pub mod reader;
pub mod scanner;
pub mod structs;

use fileresque_core::{error::AppError, types::DeletedFileEntry};
use tokio::sync::mpsc;

use reader::BlockReader;
use scanner::{omap_lookup, parse_apfs_superblock, parse_nx_superblock, walk_fs_btree};

/// Async entry point: scan an APFS container for deleted file entries.
///
/// `disk_id` is the OS-assigned identifier from [`DiskInfo`] (e.g. `"disk0"`).
/// Results are sent through `tx`; the channel is closed when scanning
/// completes or an unrecoverable error occurs.
///
/// Disk I/O is executed on a blocking thread via `spawn_blocking`.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] if Full Disk Access is not granted
/// - [`AppError::Internal`] for parse failures or `spawn_blocking` panics
pub async fn scan_apfs(
    disk_id: String,
    tx: mpsc::Sender<DeletedFileEntry>,
) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || scan_apfs_sync(&disk_id, &tx))
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking panic: {e}")))?
}

/// Synchronous APFS scan — called from [`scan_apfs`] via `spawn_blocking`.
///
/// Opens `/dev/rdiskN`, reads the container superblock, then walks each APFS
/// volume's file-system B-tree collecting deleted inode records.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] if the raw device cannot be opened
/// - [`AppError::Internal`] on malformed on-disk structures
pub fn scan_apfs_sync(
    disk_id: &str,
    tx: &mpsc::Sender<DeletedFileEntry>,
) -> Result<(), AppError> {
    let raw_device = disk_id_to_raw_device(disk_id)?;
    let mut reader = BlockReader::open(&raw_device)?;

    // Block 0 is always the container superblock
    let block0 = reader.read_block(0)?;
    let nx_sb = parse_nx_superblock(&block0)?;

    // Update the reader's block size from the parsed superblock
    reader.block_size = nx_sb.block_size;

    for &fs_oid in &nx_sb.fs_oids {
        if let Err(e) = scan_volume(&mut reader, &nx_sb, fs_oid, tx) {
            // Log individual volume failures but continue with remaining volumes
            eprintln!("[FileResque] APFS volume OID {fs_oid} scan error: {e}");
        }
    }

    Ok(())
}

/// Scan a single APFS volume identified by `fs_oid`.
fn scan_volume(
    reader: &mut BlockReader,
    nx_sb: &structs::NxSuperblock,
    fs_oid: u64,
    tx: &mpsc::Sender<DeletedFileEntry>,
) -> Result<(), AppError> {
    // Resolve volume superblock block address from container OMap
    let vol_block = omap_lookup(reader, nx_sb.omap_oid, fs_oid)?;
    let vol_buf = reader.read_block(vol_block)?;
    let apfs_sb = parse_apfs_superblock(&vol_buf)?;

    // Resolve root FS B-tree block address from volume OMap
    let root_block = omap_lookup(reader, apfs_sb.omap_oid, apfs_sb.root_tree_oid)?;

    walk_fs_btree(reader, root_block, tx)
}

/// Convert a disk identifier string to a raw device path.
///
/// `"disk0"` → `"/dev/rdisk0"`, `"disk12"` → `"/dev/rdisk12"`
///
/// # Errors
///
/// Returns `Err(AppError::Internal)` when the input does not match the pattern
/// `disk` followed by one or more ASCII digits.
pub(crate) fn disk_id_to_raw_device(disk_id: &str) -> Result<String, AppError> {
    const PREFIX: &str = "disk";

    if !disk_id.starts_with(PREFIX) || disk_id.len() <= PREFIX.len() {
        return Err(AppError::Internal(format!(
            "Invalid disk_id (must start with 'disk' and have a numeric suffix): {disk_id}"
        )));
    }

    let suffix = &disk_id[PREFIX.len()..];
    if !suffix.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::Internal(format!(
            "Non-numeric disk suffix in: {disk_id}"
        )));
    }

    Ok(format!("/dev/rdisk{suffix}"))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct RawDeviceCase {
        name: &'static str,
        input: &'static str,
        expected: Result<&'static str, ()>,
    }

    #[test]
    fn test_disk_id_to_raw_device() {
        let cases = vec![
            RawDeviceCase {
                name: "disk_id_to_raw_device_valid",
                input: "disk0",
                expected: Ok("/dev/rdisk0"),
            },
            RawDeviceCase {
                name: "disk_id_to_raw_device_multi_digit",
                input: "disk12",
                expected: Ok("/dev/rdisk12"),
            },
            RawDeviceCase {
                name: "disk_id_to_raw_device_invalid_no_prefix",
                input: "sda",
                expected: Err(()),
            },
            RawDeviceCase {
                name: "disk_id_to_raw_device_non_numeric_suffix",
                input: "diskX",
                expected: Err(()),
            },
            RawDeviceCase {
                name: "disk_id_to_raw_device_empty",
                input: "",
                expected: Err(()),
            },
            RawDeviceCase {
                name: "disk_id_to_raw_device_prefix_only",
                input: "disk",
                expected: Err(()),
            },
        ];

        for case in cases {
            let actual = disk_id_to_raw_device(case.input);
            match case.expected {
                Ok(expected_path) => {
                    let path = actual
                        // JUSTIFIED: test-only; failure message identifies which case failed
                        .unwrap_or_else(|e| panic!("FAILED case: {} — {e}", case.name));
                    assert_eq!(path, expected_path, "FAILED case: {}", case.name);
                }
                Err(()) => {
                    assert!(
                        actual.is_err(),
                        "FAILED case: {} — expected Err, got Ok({:?})",
                        case.name,
                        actual.ok()
                    );
                }
            }
        }
    }
}
