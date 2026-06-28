use fileresque_core::{error::AppError, types::DeletedFileEntry};
use std::io::{Read, Seek, SeekFrom};
use std::time::{Duration, UNIX_EPOCH};
use tokio::sync::mpsc;

mod scanner;
mod structs;

use scanner::{read_extents_from_device, walk_catalog_leaves};
use structs::{parse_volume_header, CatalogDeletedFile};

/// HFS+ epoch is 1904-01-01 00:00:00 UTC.
/// Unix epoch (1970-01-01) = HFS+ epoch + 2 082 844 800 seconds.
const HFS_TO_UNIX_OFFSET: u64 = 2_082_844_800;

/// Convert a `CatalogDeletedFile` to a `DeletedFileEntry`.
fn to_deleted_entry(file: CatalogDeletedFile) -> DeletedFileEntry {
    let deleted_at = file.mod_date.and_then(|hfs_time| {
        let unix_secs = u64::from(hfs_time).checked_sub(HFS_TO_UNIX_OFFSET)?;
        UNIX_EPOCH.checked_add(Duration::from_secs(unix_secs))
    });

    DeletedFileEntry {
        inode_id: u64::from(file.cnid),
        name: file.name,
        size_bytes: file.data_size,
        deleted_at,
        extents: file
            .extents
            .into_iter()
            .map(|(start, count)| (u64::from(start), u64::from(count)))
            .collect(),
        filesystem: fileresque_core::types::FileSystem::HFSPlus,
    }
}

/// Synchronous HFS+ scan. Opens the raw device, reads the volume header,
/// reads the allocation bitmap, walks catalog leaf nodes, and sends each
/// candidate deleted file over `tx`.
///
/// # Errors
///
/// Returns `AppError` if the device cannot be opened, the volume header is
/// invalid, or any I/O operation fails.
pub fn scan_hfsplus_sync(
    disk_id: &str,
    tx: &mpsc::Sender<DeletedFileEntry>,
) -> Result<(), AppError> {
    let raw_device = format!("/dev/r{disk_id}");
    let mut device = std::fs::File::open(&raw_device).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            AppError::PermissionDenied(raw_device.clone())
        } else {
            AppError::Io(e)
        }
    })?;

    // HFS+ Volume Header is always at byte offset 1024.
    // Read 512 bytes starting at offset 1024.
    device
        .seek(SeekFrom::Start(1024))
        .map_err(AppError::Io)?;
    let mut header_buf = vec![0u8; 512];
    device
        .read_exact(&mut header_buf)
        .map_err(AppError::Io)?;

    let header = parse_volume_header(&header_buf)?;

    if header.block_size == 0 {
        return Err(AppError::Internal(
            "HFS+ block_size reported as zero".to_string(),
        ));
    }

    // Read the allocation bitmap from the device.
    let alloc_data =
        read_extents_from_device(&mut device, &header.alloc_extents, header.block_size)?;

    // Read the catalog B*-tree data.
    let catalog_data =
        read_extents_from_device(&mut device, &header.catalog_extents, header.block_size)?;

    // The catalog header node (node 0) contains the BTHeaderRec at offset 14.
    // BTHeaderRec: treeDepth(2), rootNode(4), leafRecords(4), firstLeafNode(4),
    //              lastLeafNode(4), nodeSize(2), ...
    // We need nodeSize at offset 14 + 12 = 26 within the catalog data.
    let node_size = parse_catalog_node_size(&catalog_data)?;

    let candidates = walk_catalog_leaves(&catalog_data, node_size, &alloc_data)?;

    for file in candidates {
        let entry = to_deleted_entry(file);
        // If the receiver has dropped, stop scanning (not an error).
        if tx.blocking_send(entry).is_err() {
            break;
        }
    }

    Ok(())
}

/// Extract the B*-tree node size from the catalog header node.
/// The header node is node 0; the BTHeaderRec starts at offset 14.
/// nodeSize is a u16 at BTHeaderRec offset 12 (absolute offset 26).
fn parse_catalog_node_size(catalog_data: &[u8]) -> Result<u32, AppError> {
    // Minimum: 14 (BTNodeDescriptor) + 14 (BTHeaderRec fields up to nodeSize) = 28 bytes
    if catalog_data.len() < 28 {
        return Err(AppError::Internal(
            "Catalog data too small for header node".to_string(),
        ));
    }
    // nodeSize at absolute offset 26: BTNodeDescriptor(14) + treeDepth(2) + rootNode(4) +
    //   leafRecords(4) + firstLeafNode(4) + lastLeafNode(4) + nodeSize(2)
    // = 14 + 2 + 4 + 4 + 4 + 4 = 32 ... wait, let's recalculate:
    // BTHeaderRec fields:
    //   offset 0 (from rec start): treeDepth (u16) = 2
    //   offset 2: rootNode (u32)   = 4
    //   offset 6: leafRecords (u32) = 4
    //   offset 10: firstLeafNode (u32) = 4
    //   offset 14: lastLeafNode (u32) = 4
    //   offset 18: nodeSize (u16) = 2  → absolute: 14 + 18 = 32
    if catalog_data.len() < 34 {
        return Err(AppError::Internal(
            "Catalog data too small to read nodeSize".to_string(),
        ));
    }
    let abs_offset = 14 + 18; // = 32
    let node_size =
        u16::from_be_bytes([catalog_data[abs_offset], catalog_data[abs_offset + 1]]);

    if node_size < 512 {
        // Fallback: HFS+ default node size is 4096; 512 is the minimum valid value.
        // A zero or tiny node size indicates a corrupt or non-HFS+ volume.
        return Err(AppError::Internal(format!(
            "HFS+ catalog node_size {node_size} is below minimum 512"
        )));
    }

    Ok(u32::from(node_size))
}

/// Async HFS+ scan. Wraps `scan_hfsplus_sync` in `tokio::task::spawn_blocking`
/// so it does not block the async executor.
///
/// # Errors
///
/// Returns `AppError` if the blocking task panics or the underlying sync scan fails.
pub async fn scan_hfsplus(
    disk_id: String,
    tx: mpsc::Sender<DeletedFileEntry>,
) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || scan_hfsplus_sync(&disk_id, &tx))
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking panic: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Case<I, O> {
        name: &'static str,
        input: I,
        expected: O,
    }

    #[test]
    fn test_to_deleted_entry() {
        use fileresque_core::types::FileSystem;

        let cases = vec![
            Case {
                name: "to_deleted_entry_with_timestamp",
                input: CatalogDeletedFile {
                    cnid: 42,
                    name: Some("test.txt".to_string()),
                    data_size: 1024,
                    mod_date: Some(0x7C25_B080),
                    extents: vec![(10, 2)],
                },
                expected: (42u64, true, 1024u64),
            },
            Case {
                name: "to_deleted_entry_no_timestamp",
                input: CatalogDeletedFile {
                    cnid: 99,
                    name: None,
                    data_size: 0,
                    mod_date: None,
                    extents: vec![],
                },
                expected: (99u64, false, 0u64),
            },
        ];

        for case in cases {
            let (expected_inode, expected_has_time, expected_size) = case.expected;
            let entry = to_deleted_entry(case.input);
            assert_eq!(
                entry.inode_id, expected_inode,
                "FAILED case: {} — inode_id",
                case.name
            );
            assert_eq!(
                entry.deleted_at.is_some(),
                expected_has_time,
                "FAILED case: {} — deleted_at",
                case.name
            );
            assert_eq!(
                entry.size_bytes, expected_size,
                "FAILED case: {} — size_bytes",
                case.name
            );
            assert!(
                matches!(entry.filesystem, FileSystem::HFSPlus),
                "FAILED case: {} — filesystem",
                case.name
            );
        }
    }

    #[test]
    fn test_parse_catalog_node_size() {
        // Build a minimal catalog header node with nodeSize = 4096 at absolute offset 32.
        let mut buf = vec![0u8; 512];
        // BTNodeDescriptor (14 bytes) all zeros = valid header node (kind=1 at offset 8)
        buf[8] = 1i8 as u8; // kind = BT_HEADER_NODE
        // BTHeaderRec starts at offset 14:
        // treeDepth(2) + rootNode(4) + leafRecords(4) + firstLeafNode(4) + lastLeafNode(4) = 18
        // nodeSize at offset 14+18 = 32
        buf[32] = 0x10; // 0x1000 = 4096
        buf[33] = 0x00;

        let cases = vec![
            Case {
                name: "parse_catalog_node_size_valid_4096",
                input: buf.clone(),
                expected: Ok(4096u32),
            },
            Case {
                name: "parse_catalog_node_size_too_small",
                input: vec![0u8; 10],
                expected: Err("too small".to_string()),
            },
        ];

        for case in cases {
            let result = parse_catalog_node_size(&case.input);
            match (result, case.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "FAILED case: {}", case.name);
                }
                (Err(_), Err(_)) => {}
                (got, expected) => {
                    panic!(
                        "FAILED case: {} — got {:?}, expected {:?}",
                        case.name, got, expected
                    );
                }
            }
        }
    }
}
