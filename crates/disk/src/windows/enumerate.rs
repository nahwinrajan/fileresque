use fileresque_core::{error::AppError, types::DiskInfo};

// DriveType and FileSystem are only referenced in cfg-gated helper functions;
// importing them unconditionally would trigger `unused_imports` on non-Windows.
#[cfg(any(target_os = "windows", test))]
use fileresque_core::types::{DriveType, FileSystem};

/// Windows disk information as returned by Win32 storage APIs.
///
/// Kept as an intermediate struct so parsing logic can be unit-tested
/// without calling Win32 APIs. Fields mirror the data available from
/// `STORAGE_DEVICE_DESCRIPTOR` and `IOCTL_DISK_GET_LENGTH_INFO`.
///
/// Only compiled on Windows production builds or on any platform during tests,
/// because it is dead code in macOS/Linux production builds.
#[cfg(any(target_os = "windows", test))]
#[derive(Debug)]
pub(crate) struct RawDiskInfo {
    /// Device path, e.g. `r"\\.\PhysicalDrive0"`.
    pub device_path: String,
    /// Product name from `STORAGE_DEVICE_DESCRIPTOR.ProductId`.
    pub friendly_name: String,
    /// Serial number from `STORAGE_DEVICE_DESCRIPTOR.SerialNumber`, if present.
    pub serial: Option<String>,
    /// Total disk size in bytes from `IOCTL_DISK_GET_LENGTH_INFO`.
    pub size_bytes: u64,
    /// Raw `STORAGE_BUS_TYPE` enum value from `STORAGE_DEVICE_DESCRIPTOR`.
    pub bus_type: u8,
    /// Whether the device is marked removable in `STORAGE_DEVICE_DESCRIPTOR`.
    pub is_removable: bool,
    /// Partition style: 0 = MBR, 1 = GPT (simplified for P1; not read on non-Windows).
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub partition_style: u8,
}

/// Enumerate physical disks visible to the OS on Windows.
///
/// Delegates synchronous Win32 I/O to a blocking thread so the async
/// executor is never stalled.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] if the process lacks Administrator privileges.
/// - [`AppError::Internal`] if the blocking task panics or Win32 calls fail.
pub async fn list_disks() -> Result<Vec<DiskInfo>, AppError> {
    tokio::task::spawn_blocking(list_disks_sync)
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking join error: {e}")))?
}

/// Synchronous disk enumeration — called from [`list_disks`] via `spawn_blocking`.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] if Administrator privileges are absent.
/// - [`AppError::Internal`] on Win32 IOCTL failures.
#[cfg(target_os = "windows")]
fn list_disks_sync() -> Result<Vec<DiskInfo>, AppError> {
    let raw_disks = super::ioctl::enumerate_physical_drives()?;
    Ok(raw_disks.into_iter().map(map_raw_to_disk_info).collect())
}

/// Non-Windows stub — returns an error so callers fail loudly rather than
/// silently returning an empty list.
///
/// # Errors
///
/// Always returns [`AppError::Internal`] on non-Windows platforms.
#[cfg(not(target_os = "windows"))]
fn list_disks_sync() -> Result<Vec<DiskInfo>, AppError> {
    Err(AppError::Internal(
        "Windows disk enumeration called on non-Windows OS".to_string(),
    ))
}

/// Convert a [`RawDiskInfo`] (Win32 data) into the crate-shared [`DiskInfo`].
///
/// Filesystem and mount-point fields are left as defaults; they are populated
/// by partition enumeration in a future task.
#[cfg(any(target_os = "windows", test))]
#[must_use]
pub(crate) fn map_raw_to_disk_info(raw: RawDiskInfo) -> DiskInfo {
    // Strip the `\\.\` device-namespace prefix so the id is "PhysicalDrive0" etc.
    let id = raw.device_path.trim_start_matches(r"\\.\").to_owned();

    DiskInfo {
        id,
        display_name: raw.friendly_name,
        size_bytes: raw.size_bytes,
        drive_type: detect_drive_type(raw.bus_type, raw.is_removable),
        // Filesystem is detected per-partition on Windows, not per physical disk.
        filesystem: FileSystem::Unknown,
        // Mount points are populated by partition enumeration (future task).
        mount_points: vec![],
        // BitLocker detection is P2+.
        encrypted: false,
        // TRIM detection via IOCTL_STORAGE_MANAGE_DATA_SET_ATTRIBUTES is future work.
        trim_enabled: false,
        serial: raw.serial,
    }
}

/// Map a Win32 `STORAGE_BUS_TYPE` value (plus the removable flag) to [`DriveType`].
///
/// Reference: <https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddstor/ne-ntddstor-storage_bus_type>
///
/// The removable flag is checked first because USB flash drives sometimes report
/// a non-USB bus type when connected via a hub.
///
/// Mapping:
/// - `BusTypeUsb = 7`
/// - `BusTypeAta = 3`
/// - `BusTypeSata = 11 (0xB)`
/// - `BusTypeNvme = 17 (0x11)`
/// - `BusTypeVirtual = 14 (0xE)`
/// - `BusTypeFileBackedVirtual = 15 (0xF)`
#[cfg(any(target_os = "windows", test))]
#[must_use]
pub(crate) fn detect_drive_type(bus_type: u8, is_removable: bool) -> DriveType {
    if is_removable {
        return DriveType::USB;
    }
    match bus_type {
        7 => DriveType::USB,
        17 => DriveType::NVMe,
        // ATA (3) and SATA (11) are spinning-disk protocols on Windows.
        3 | 11 => DriveType::HDD,
        14 | 15 => DriveType::Virtual,
        _ => DriveType::Unknown,
    }
}

/// Extract a null-terminated ASCII string from `buf` at `offset`.
///
/// Returns `None` when:
/// - `offset == 0` — Win32 uses 0 as a sentinel meaning "field not present" in
///   `STORAGE_DEVICE_DESCRIPTOR`.
/// - `offset >= buf.len()` — out-of-bounds guard.
/// - The string at `offset` is empty after trimming ASCII whitespace.
#[cfg(any(target_os = "windows", test))]
#[must_use]
pub(crate) fn extract_offset_string(buf: &[u8], offset: usize) -> Option<String> {
    if offset == 0 || offset >= buf.len() {
        return None;
    }
    let slice = &buf[offset..];
    let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    let s = String::from_utf8_lossy(&slice[..end]).trim().to_owned();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // detect_drive_type — 7 cases covering every match arm + removable flag
    // -------------------------------------------------------------------------

    #[derive(Debug)]
    struct DriveTypeCase {
        name: &'static str,
        bus_type: u8,
        is_removable: bool,
        expected_debug: &'static str,
    }

    #[test]
    fn test_detect_drive_type() {
        let cases = vec![
            DriveTypeCase {
                name: "detect_drive_type_usb_removable_flag",
                bus_type: 11, // SATA bus type — but removable flag wins
                is_removable: true,
                expected_debug: "USB",
            },
            DriveTypeCase {
                name: "detect_drive_type_usb_bus_type",
                bus_type: 7,
                is_removable: false,
                expected_debug: "USB",
            },
            DriveTypeCase {
                name: "detect_drive_type_nvme",
                bus_type: 17,
                is_removable: false,
                expected_debug: "NVMe",
            },
            DriveTypeCase {
                name: "detect_drive_type_sata_hdd",
                bus_type: 11,
                is_removable: false,
                expected_debug: "HDD",
            },
            DriveTypeCase {
                name: "detect_drive_type_ata_hdd",
                bus_type: 3,
                is_removable: false,
                expected_debug: "HDD",
            },
            DriveTypeCase {
                name: "detect_drive_type_virtual",
                bus_type: 14,
                is_removable: false,
                expected_debug: "Virtual",
            },
            DriveTypeCase {
                name: "detect_drive_type_unknown_bus",
                bus_type: 99,
                is_removable: false,
                expected_debug: "Unknown",
            },
        ];

        for case in cases {
            let actual = detect_drive_type(case.bus_type, case.is_removable);
            assert_eq!(
                format!("{actual:?}"),
                case.expected_debug,
                "FAILED case: {}",
                case.name
            );
        }
    }

    // -------------------------------------------------------------------------
    // map_raw_to_disk_info — 6 cases covering field mapping
    // -------------------------------------------------------------------------

    fn make_raw(device_path: &str, size_bytes: u64, serial: Option<&str>) -> RawDiskInfo {
        RawDiskInfo {
            device_path: device_path.to_owned(),
            friendly_name: "Test Disk".to_owned(),
            serial: serial.map(str::to_owned),
            size_bytes,
            bus_type: 17, // NVMe
            is_removable: false,
            partition_style: 1,
        }
    }

    #[test]
    fn test_map_raw_to_disk_info() {
        #[derive(Debug)]
        enum Check {
            Id(&'static str),
            SizeBytes(u64),
            FilesystemUnknown,
            SerialNone,
            SerialSome(&'static str),
        }

        #[derive(Debug)]
        struct Case {
            name: &'static str,
            raw: RawDiskInfo,
            check: Check,
        }

        let cases = vec![
            Case {
                name: "map_raw_id_strips_prefix",
                raw: make_raw(r"\\.\PhysicalDrive0", 0, None),
                check: Check::Id("PhysicalDrive0"),
            },
            Case {
                name: "map_raw_id_strips_prefix_drive3",
                raw: make_raw(r"\\.\PhysicalDrive3", 0, None),
                check: Check::Id("PhysicalDrive3"),
            },
            Case {
                name: "map_raw_size_preserved",
                raw: make_raw(r"\\.\PhysicalDrive0", 1_000_000_000, None),
                check: Check::SizeBytes(1_000_000_000),
            },
            Case {
                name: "map_raw_filesystem_unknown",
                raw: make_raw(r"\\.\PhysicalDrive0", 0, None),
                check: Check::FilesystemUnknown,
            },
            Case {
                name: "map_raw_serial_none",
                raw: make_raw(r"\\.\PhysicalDrive0", 0, None),
                check: Check::SerialNone,
            },
            Case {
                name: "map_raw_serial_some",
                raw: make_raw(r"\\.\PhysicalDrive0", 0, Some("S123ABC")),
                check: Check::SerialSome("S123ABC"),
            },
        ];

        for case in cases {
            let info = map_raw_to_disk_info(case.raw);
            match case.check {
                Check::Id(expected) => assert_eq!(info.id, expected, "FAILED case: {}", case.name),
                Check::SizeBytes(expected) => {
                    assert_eq!(info.size_bytes, expected, "FAILED case: {}", case.name);
                }
                Check::FilesystemUnknown => assert_eq!(
                    format!("{:?}", info.filesystem),
                    "Unknown",
                    "FAILED case: {}",
                    case.name
                ),
                Check::SerialNone => assert!(
                    info.serial.is_none(),
                    "FAILED case: {} — expected serial=None, got {:?}",
                    case.name,
                    info.serial
                ),
                Check::SerialSome(expected) => assert_eq!(
                    info.serial.as_deref(),
                    Some(expected),
                    "FAILED case: {}",
                    case.name
                ),
            }
        }
    }

    // -------------------------------------------------------------------------
    // extract_offset_string — 5 cases covering every return path
    // -------------------------------------------------------------------------

    #[derive(Debug)]
    struct ExtractCase {
        name: &'static str,
        buf: Vec<u8>,
        offset: usize,
        expected: Option<&'static str>,
    }

    #[test]
    fn test_extract_offset_string() {
        let cases = vec![
            ExtractCase {
                // offset=1 skips the leading preamble byte, reaching "Samsung SSD\0"
                name: "extract_offset_string_valid",
                buf: b"\xFFSamsung SSD\x00trailing".to_vec(),
                offset: 1,
                expected: Some("Samsung SSD"),
            },
            ExtractCase {
                // Win32 uses offset=0 as the "field not present" sentinel in
                // STORAGE_DEVICE_DESCRIPTOR; always return None for offset=0.
                name: "extract_offset_string_zero_offset",
                buf: b"Samsung SSD\x00".to_vec(),
                offset: 0,
                expected: None,
            },
            ExtractCase {
                name: "extract_offset_string_out_of_bounds",
                buf: b"hello".to_vec(),
                offset: 100,
                expected: None,
            },
            ExtractCase {
                // Null byte at offset produces an empty string, collapsed to None.
                name: "extract_offset_string_empty_at_offset",
                buf: b"\xFF\x00".to_vec(),
                offset: 1,
                expected: None,
            },
            ExtractCase {
                // Whitespace-only content is trimmed to empty, collapsed to None.
                name: "extract_offset_string_whitespace_only",
                buf: b"\xFF   \x00".to_vec(),
                offset: 1,
                expected: None,
            },
        ];

        for case in cases {
            let actual = extract_offset_string(&case.buf, case.offset);
            assert_eq!(
                actual.as_deref(),
                case.expected,
                "FAILED case: {}",
                case.name
            );
        }
    }
}
