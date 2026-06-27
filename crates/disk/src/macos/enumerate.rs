use std::process::Command;

use fileresque_core::{
    error::AppError,
    types::{DiskInfo, DriveType, FileSystem},
};
use plist::{Dictionary, Value};

/// Enumerate all physical disks on macOS using `diskutil`.
///
/// Delegates the synchronous disk I/O to a blocking thread so the async
/// executor is never stalled.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] if Full Disk Access is not granted
/// - [`AppError::Internal`] if `diskutil` output cannot be parsed or the
///   blocking task is cancelled
pub async fn list_disks() -> Result<Vec<DiskInfo>, AppError> {
    tokio::task::spawn_blocking(list_disks_sync)
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking join error: {e}")))?
}

/// Synchronous disk enumeration — called from [`list_disks`] via
/// `spawn_blocking`.
///
/// Uses `diskutil list -plist physical` to discover whole-disk identifiers,
/// then `diskutil info -plist /dev/<id>` for per-disk metadata.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] if `diskutil` is denied access to a disk
/// - [`AppError::Internal`] on process or parse failures
pub(crate) fn list_disks_sync() -> Result<Vec<DiskInfo>, AppError> {
    let output = run_diskutil(&["list", "-plist", "physical"])?;
    let plist_value = parse_plist(&output)?;
    let disk_ids = extract_physical_disk_ids(&plist_value)?;

    let mut disks = Vec::new();
    for disk_id in disk_ids {
        match disk_info_for(&disk_id) {
            Ok(info) => disks.push(info),
            Err(AppError::PermissionDenied(_)) => {
                return Err(AppError::PermissionDenied(disk_id));
            }
            // Skip disks that error mid-enumeration (e.g. ejected between list and info).
            Err(_) => {}
        }
    }
    Ok(disks)
}

fn run_diskutil(args: &[&str]) -> Result<Vec<u8>, AppError> {
    let output = Command::new("diskutil")
        .args(args)
        .output()
        .map_err(|e| AppError::Internal(format!("diskutil exec failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);
        return Err(classify_stderr_error(&stderr, exit_code));
    }
    Ok(output.stdout)
}

/// Map a `diskutil` non-zero exit to the appropriate [`AppError`] variant.
///
/// Returns [`AppError::PermissionDenied`] when stderr indicates a permissions
/// problem, or [`AppError::Internal`] otherwise.
///
/// Extracted as `pub(crate)` so unit tests can verify error classification
/// without spawning a real `diskutil` process.
pub(crate) fn classify_stderr_error(stderr: &str, exit_code: i32) -> AppError {
    if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") {
        AppError::PermissionDenied("diskutil requires Full Disk Access".to_string())
    } else {
        AppError::Internal(format!("diskutil exited {exit_code}: {stderr}"))
    }
}

fn parse_plist(data: &[u8]) -> Result<Value, AppError> {
    plist::from_bytes(data).map_err(|e| AppError::Internal(format!("plist parse error: {e}")))
}

fn extract_physical_disk_ids(plist_value: &Value) -> Result<Vec<String>, AppError> {
    let dict = plist_value
        .as_dictionary()
        .ok_or_else(|| AppError::Internal("diskutil list output is not a dict".to_string()))?;

    let whole_disks = dict
        .get("WholeDisks")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Internal("WholeDisks key missing".to_string()))?;

    Ok(whole_disks
        .iter()
        .filter_map(Value::as_string)
        .map(str::to_owned)
        .collect())
}

fn disk_info_for(disk_id: &str) -> Result<DiskInfo, AppError> {
    let path = format!("/dev/{disk_id}");
    let output = run_diskutil(&["info", "-plist", &path])?;
    let plist_value = parse_plist(&output)?;
    let dict = plist_value
        .as_dictionary()
        .ok_or_else(|| AppError::Internal(format!("diskutil info for {disk_id} is not a dict")))?;
    Ok(parse_disk_info_from_dict(disk_id, dict))
}

/// Build a [`DiskInfo`] from a parsed `diskutil info -plist` dictionary.
///
/// Exported as `pub(crate)` so unit tests can supply in-memory dictionaries
/// without invoking `diskutil`.
pub(crate) fn parse_disk_info_from_dict(disk_id: &str, dict: &Dictionary) -> DiskInfo {
    let display_name = dict
        .get("MediaName")
        .and_then(Value::as_string)
        .unwrap_or(disk_id)
        .to_owned();

    let size_bytes = dict
        .get("TotalSize")
        .and_then(Value::as_unsigned_integer)
        .unwrap_or_default();

    let encrypted = dict
        .get("Encryption")
        .and_then(Value::as_boolean)
        .unwrap_or_default();

    let trim_enabled = dict
        .get("TrimSupport")
        .and_then(Value::as_boolean)
        .unwrap_or_default();

    let mount_points = dict
        .get("MountPoint")
        .and_then(Value::as_string)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .into_iter()
        .collect();

    let serial = dict
        .get("IORegistryEntryName")
        .and_then(Value::as_string)
        .map(str::to_owned);

    let filesystem = detect_filesystem(dict);
    let drive_type = detect_drive_type(dict);

    DiskInfo {
        id: disk_id.to_owned(),
        display_name,
        size_bytes,
        drive_type,
        filesystem,
        mount_points,
        encrypted,
        trim_enabled,
        serial,
    }
}

fn detect_filesystem(dict: &Dictionary) -> FileSystem {
    let fs_type = dict
        .get("FilesystemType")
        .and_then(Value::as_string)
        .unwrap_or_default();
    let fs_name = dict
        .get("FilesystemName")
        .and_then(Value::as_string)
        .unwrap_or_default();

    // Combine and lowercase so a single pass handles all diskutil key variants.
    let combined = format!("{fs_type} {fs_name}").to_lowercase();

    if combined.contains("apfs") {
        FileSystem::APFS
    } else if combined.contains("hfs") || combined.contains("mac os extended") {
        FileSystem::HFSPlus
    } else if combined.contains("fat32") || combined.contains("msdos") {
        FileSystem::FAT32
    } else if combined.contains("exfat") {
        FileSystem::ExFAT
    } else if combined.contains("ntfs") {
        FileSystem::NTFS
    } else {
        FileSystem::Unknown
    }
}

fn detect_drive_type(dict: &Dictionary) -> DriveType {
    let solid_state = dict
        .get("SolidState")
        .and_then(Value::as_boolean)
        .unwrap_or_default();

    let protocol = dict
        .get("BusProtocol")
        .and_then(Value::as_string)
        .unwrap_or_default()
        .to_lowercase();

    if protocol.contains("usb") {
        DriveType::USB
    } else if protocol.contains("nvme") || protocol.contains("pcie") {
        DriveType::NVMe
    } else if solid_state {
        DriveType::SSD
    } else if protocol.contains("sata") || protocol.contains("ata") {
        DriveType::HDD
    } else {
        DriveType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plist::{Dictionary, Value};

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    /// Build a `plist::Value::Dictionary` from `(key, Value)` pairs.
    fn make_plist_value(pairs: Vec<(&str, Value)>) -> Value {
        let mut dict = Dictionary::new();
        for (key, val) in pairs {
            dict.insert(key.to_owned(), val);
        }
        Value::Dictionary(dict)
    }

    /// Build a `plist::Dictionary` from `(key, Value)` pairs.
    fn make_dict(pairs: Vec<(&str, Value)>) -> Dictionary {
        let mut dict = Dictionary::new();
        for (key, val) in pairs {
            dict.insert(key.to_owned(), val);
        }
        dict
    }

    // ---------------------------------------------------------------------------
    // extract_physical_disk_ids — 2 cases
    // ---------------------------------------------------------------------------

    #[derive(Debug)]
    struct DiskIdCase {
        name: &'static str,
        whole_disks: Vec<&'static str>,
        expected: Vec<String>,
    }

    #[test]
    fn test_extract_physical_disk_ids() {
        let cases = vec![
            DiskIdCase {
                name: "parse_whole_disks_empty",
                whole_disks: vec![],
                expected: vec![],
            },
            DiskIdCase {
                name: "parse_whole_disks_single",
                whole_disks: vec!["disk0"],
                expected: vec!["disk0".to_owned()],
            },
        ];

        for case in cases {
            let array = case
                .whole_disks
                .iter()
                .map(|s| Value::String((*s).to_owned()))
                .collect::<Vec<_>>();
            let plist_val = make_plist_value(vec![("WholeDisks", Value::Array(array))]);
            let actual = extract_physical_disk_ids(&plist_val)
                // JUSTIFIED: test-only; fixture plist is always valid
                .expect("extract_physical_disk_ids must not fail on valid fixture");
            assert_eq!(actual, case.expected, "FAILED case: {}", case.name);
        }
    }

    // ---------------------------------------------------------------------------
    // detect_filesystem — 5 cases
    // ---------------------------------------------------------------------------

    #[derive(Debug)]
    struct FilesystemCase {
        name: &'static str,
        fs_type: &'static str,
        fs_name: &'static str,
        expected_debug: &'static str,
    }

    #[test]
    fn test_detect_filesystem() {
        let cases = vec![
            FilesystemCase {
                name: "detect_filesystem_apfs",
                fs_type: "apfs",
                fs_name: "",
                expected_debug: "APFS",
            },
            FilesystemCase {
                name: "detect_filesystem_hfsplus",
                fs_type: "",
                fs_name: "Mac OS Extended",
                expected_debug: "HFSPlus",
            },
            FilesystemCase {
                name: "detect_filesystem_fat32",
                fs_type: "msdos",
                fs_name: "",
                expected_debug: "FAT32",
            },
            FilesystemCase {
                name: "detect_filesystem_exfat",
                fs_type: "exfat",
                fs_name: "",
                expected_debug: "ExFAT",
            },
            FilesystemCase {
                name: "detect_filesystem_unknown",
                fs_type: "",
                fs_name: "",
                expected_debug: "Unknown",
            },
        ];

        for case in cases {
            let mut pairs: Vec<(&str, Value)> = vec![];
            if !case.fs_type.is_empty() {
                pairs.push(("FilesystemType", Value::String(case.fs_type.to_owned())));
            }
            if !case.fs_name.is_empty() {
                pairs.push(("FilesystemName", Value::String(case.fs_name.to_owned())));
            }
            let dict = make_dict(pairs);
            let actual = detect_filesystem(&dict);
            assert_eq!(
                format!("{actual:?}"),
                case.expected_debug,
                "FAILED case: {}",
                case.name
            );
        }
    }

    // ---------------------------------------------------------------------------
    // detect_drive_type — 5 cases
    // ---------------------------------------------------------------------------

    #[derive(Debug)]
    struct DriveTypeCase {
        name: &'static str,
        bus_protocol: &'static str,
        solid_state: bool,
        expected_debug: &'static str,
    }

    #[test]
    fn test_detect_drive_type() {
        let cases = vec![
            DriveTypeCase {
                name: "detect_drive_type_usb",
                bus_protocol: "USB",
                solid_state: false,
                expected_debug: "USB",
            },
            DriveTypeCase {
                name: "detect_drive_type_nvme",
                bus_protocol: "NVMe",
                solid_state: false,
                expected_debug: "NVMe",
            },
            DriveTypeCase {
                name: "detect_drive_type_ssd",
                bus_protocol: "",
                solid_state: true,
                expected_debug: "SSD",
            },
            DriveTypeCase {
                name: "detect_drive_type_hdd",
                bus_protocol: "SATA",
                solid_state: false,
                expected_debug: "HDD",
            },
            DriveTypeCase {
                name: "detect_drive_type_unknown",
                bus_protocol: "",
                solid_state: false,
                expected_debug: "Unknown",
            },
        ];

        for case in cases {
            let mut pairs: Vec<(&str, Value)> = vec![];
            if !case.bus_protocol.is_empty() {
                pairs.push(("BusProtocol", Value::String(case.bus_protocol.to_owned())));
            }
            if case.solid_state {
                pairs.push(("SolidState", Value::Boolean(true)));
            }
            let dict = make_dict(pairs);
            let actual = detect_drive_type(&dict);
            assert_eq!(
                format!("{actual:?}"),
                case.expected_debug,
                "FAILED case: {}",
                case.name
            );
        }
    }

    // ---------------------------------------------------------------------------
    // parse_disk_info_from_dict — 2 cases (encrypted, trim)
    // ---------------------------------------------------------------------------

    /// Which boolean field of [`DiskInfo`] to assert on.
    #[derive(Debug)]
    enum DiskInfoField {
        Encrypted,
        Trim,
    }

    #[derive(Debug)]
    struct DiskInfoFieldCase {
        name: &'static str,
        encryption: bool,
        trim_support: bool,
        /// Which field to read from the resulting [`DiskInfo`].
        check_field: DiskInfoField,
        expected: bool,
    }

    #[test]
    fn test_disk_info_fields() {
        let cases = vec![
            DiskInfoFieldCase {
                name: "disk_info_encrypted",
                encryption: true,
                trim_support: false,
                check_field: DiskInfoField::Encrypted,
                expected: true,
            },
            DiskInfoFieldCase {
                name: "disk_info_trim",
                encryption: false,
                trim_support: true,
                check_field: DiskInfoField::Trim,
                expected: true,
            },
        ];

        for case in cases {
            let dict = make_dict(vec![
                ("Encryption", Value::Boolean(case.encryption)),
                ("TrimSupport", Value::Boolean(case.trim_support)),
            ]);
            let info = parse_disk_info_from_dict("disk0", &dict);
            let actual = match case.check_field {
                DiskInfoField::Encrypted => info.encrypted,
                DiskInfoField::Trim => info.trim_enabled,
            };
            assert_eq!(actual, case.expected, "FAILED case: {}", case.name);
        }
    }

    // ---------------------------------------------------------------------------
    // classify_stderr_error — 1 case
    // ---------------------------------------------------------------------------

    #[test]
    fn test_classify_stderr_error() {
        #[derive(Debug)]
        struct Case {
            name: &'static str,
            stderr: &'static str,
            expected_permission_denied: bool,
        }

        let cases = vec![Case {
            name: "run_diskutil_permission_error",
            stderr: "Operation not permitted",
            expected_permission_denied: true,
        }];

        for case in cases {
            let actual = classify_stderr_error(case.stderr, 1);
            let is_permission_denied = matches!(actual, AppError::PermissionDenied(_));
            assert_eq!(
                is_permission_denied, case.expected_permission_denied,
                "FAILED case: {}",
                case.name
            );
        }
    }

    // ---------------------------------------------------------------------------
    // parse_plist — 1 case (invalid input)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_parse_plist_invalid() {
        #[derive(Debug)]
        struct Case {
            name: &'static str,
            input: Vec<u8>,
            expected_is_internal_err: bool,
        }

        let cases = vec![Case {
            name: "parse_plist_invalid",
            input: vec![],
            expected_is_internal_err: true,
        }];

        for case in cases {
            let result = parse_plist(&case.input);
            let is_internal_err = matches!(result, Err(AppError::Internal(_)));
            assert_eq!(
                is_internal_err, case.expected_is_internal_err,
                "FAILED case: {}",
                case.name
            );
        }
    }
}
