use byteorder::{ReadBytesExt, BE};
use fileresque_core::error::AppError;
use std::io::{Cursor, Read, Seek, SeekFrom};

use super::structs::{
    is_block_allocated, BT_LEAF_NODE, HFS_FILE_RECORD, HFS_FILE_THREAD_RECORD,
};
pub(crate) use super::structs::CatalogDeletedFile;

/// Offset of modifyDate within an HFS+ catalog file record (after the 2-byte record type).
/// HFSPlusCatalogFile layout (offsets from record start):
///   0   recordType (i16)
///   2   flags (u16)
///   4   reserved1 (u32)
///   8   fileID / CNID (u32)
///  12   createDate (u32)
///  16   contentModDate (u32)
///  ...
///  48   dataFork.logicalSize (u64)
///  ...
///  88   dataFork.extents (8 × 8 bytes)
const FILE_REC_FLAGS_OFFSET: u64 = 2;
const FILE_REC_CNID_OFFSET: u64 = 8;
const FILE_REC_MODIFY_DATE_OFFSET: u64 = 16;
const FILE_REC_DATA_LOGICAL_SIZE_OFFSET: u64 = 48;
const FILE_REC_DATA_EXTENTS_OFFSET: u64 = 88;
const FILE_REC_MIN_SIZE: u64 = 88 + 64; // extents array ends at 88 + 8*8 = 152

/// Read blocks described by a list of extents from an open device file.
/// Returns a contiguous byte vector of all extent data concatenated.
pub(crate) fn read_extents_from_device(
    device: &mut std::fs::File,
    extents: &[(u32, u32)],
    block_size: u32,
) -> Result<Vec<u8>, AppError> {
    if block_size == 0 {
        return Err(AppError::Internal(
            "HFS+ block_size is zero".to_string(),
        ));
    }
    let bs = u64::from(block_size);
    let mut data = Vec::new();
    for &(start_block, block_count) in extents {
        if block_count == 0 {
            continue;
        }
        let offset = u64::from(start_block) * bs;
        let length = u64::from(block_count) * bs;
        device
            .seek(SeekFrom::Start(offset))
            .map_err(AppError::Io)?;
        let prev_len = data.len();
        let length_usize = usize::try_from(length)
            .map_err(|_| AppError::Internal("extent too large for address space".to_string()))?;
        data.resize(prev_len + length_usize, 0u8);
        device
            .read_exact(&mut data[prev_len..])
            .map_err(AppError::Io)?;
    }
    Ok(data)
}

/// Parse a single HFS+ catalog file record from `record_buf`.
/// Returns `None` if the record is not a file record or is malformed.
fn parse_file_record(record_buf: &[u8]) -> Option<CatalogDeletedFile> {
    if u64::try_from(record_buf.len()).ok()? < FILE_REC_MIN_SIZE {
        return None;
    }
    let mut cur = Cursor::new(record_buf);

    // offset 0: recordType
    let record_type = cur.read_i16::<BE>().ok()?;
    if record_type != HFS_FILE_RECORD {
        return None;
    }

    // offset 2: flags (skip)
    cur.seek(SeekFrom::Start(FILE_REC_FLAGS_OFFSET + 2))
        .ok()?;

    // offset 8: fileID (CNID)
    cur.seek(SeekFrom::Start(FILE_REC_CNID_OFFSET)).ok()?;
    let cnid = cur.read_u32::<BE>().ok()?;

    // offset 16: contentModDate
    cur.seek(SeekFrom::Start(FILE_REC_MODIFY_DATE_OFFSET))
        .ok()?;
    let mod_date = cur.read_u32::<BE>().ok()?;

    // offset 48: dataFork.logicalSize
    cur.seek(SeekFrom::Start(FILE_REC_DATA_LOGICAL_SIZE_OFFSET))
        .ok()?;
    let data_size = cur.read_u64::<BE>().ok()?;

    // offset 88: dataFork.extents (8 ExtentDescriptors × 8 bytes)
    cur.seek(SeekFrom::Start(FILE_REC_DATA_EXTENTS_OFFSET))
        .ok()?;
    let mut extents = Vec::with_capacity(8);
    for _ in 0..8 {
        let start = cur.read_u32::<BE>().ok()?;
        let count = cur.read_u32::<BE>().ok()?;
        if start != 0 || count != 0 {
            extents.push((start, count));
        }
    }

    let mod_date_opt = if mod_date == 0 { None } else { Some(mod_date) };

    Some(CatalogDeletedFile {
        cnid,
        name: None, // name is in the key, not the record; caller may set this
        data_size,
        mod_date: mod_date_opt,
        extents,
    })
}

/// Walk all leaf nodes in a flat catalog B*-tree byte buffer.
/// Collects file records whose extents include at least one free block
/// (indicating a candidate deleted file).
pub(crate) fn walk_catalog_leaves(
    catalog_data: &[u8],
    node_size: u32,
    alloc_bitmap: &[u8],
) -> Result<Vec<CatalogDeletedFile>, AppError> {
    if node_size == 0 {
        return Err(AppError::Internal(
            "HFS+ B*-tree node_size is zero".to_string(),
        ));
    }

    let ns = node_size as usize;
    let mut candidates: Vec<CatalogDeletedFile> = Vec::new();

    // Iterate over each node in the catalog data
    let node_count = catalog_data.len() / ns;
    for node_idx in 0..node_count {
        let node_start = node_idx * ns;
        let node_buf = &catalog_data[node_start..node_start + ns];

        let result = process_catalog_node(node_buf, ns, alloc_bitmap);
        if let Ok(mut files) = result {
            candidates.append(&mut files);
        }
    }

    Ok(candidates)
}

/// Process a single B*-tree node buffer, returning any deleted file candidates.
fn process_catalog_node(
    node_buf: &[u8],
    node_size: usize,
    alloc_bitmap: &[u8],
) -> Result<Vec<CatalogDeletedFile>, AppError> {
    if node_buf.len() < node_size {
        return Err(AppError::Internal("Node buffer too small".to_string()));
    }

    let mut cur = Cursor::new(node_buf);

    // BTNodeDescriptor: fLink(4) + bLink(4) + kind(1) + height(1) + numRecords(2) + reserved(2)
    // offset 0: fLink (skip)
    // offset 4: bLink (skip)
    cur.seek(SeekFrom::Start(8))
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let kind = cur
        .read_i8()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if kind != BT_LEAF_NODE {
        return Ok(vec![]);
    }

    // offset 9: height (skip)
    cur.seek(SeekFrom::Start(10))
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let num_records = cur
        .read_u16::<BE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if num_records == 0 {
        return Ok(vec![]);
    }

    // The offset table is at the end of the node: (num_records + 1) × 2 bytes
    // offsets[i] = u16 at position node_size - 2*(i+1)
    let mut candidates = Vec::new();
    for i in 0..num_records {
        let table_pos = node_size - 2 * (usize::from(i) + 1);
        if table_pos + 2 > node_size {
            break;
        }
        let rec_offset = u16::from_be_bytes([node_buf[table_pos], node_buf[table_pos + 1]]);
        let rec_start = rec_offset as usize;

        // Next record offset to determine record length
        let next_table_pos = node_size - 2 * (usize::from(i) + 2);
        let rec_end = if next_table_pos + 2 <= node_size && usize::from(i) + 1 < usize::from(num_records) {
            u16::from_be_bytes([node_buf[next_table_pos], node_buf[next_table_pos + 1]]) as usize
        } else {
            // Last record ends at start of offset table
            node_size - 2 * (usize::from(num_records) + 1)
        };

        if rec_start >= rec_end || rec_end > node_size {
            continue;
        }

        // Each leaf node record has a key followed by the data record.
        // We skip the key: first 2 bytes are keyLength (u16 BE), then keyLength bytes of key.
        let rec_buf = &node_buf[rec_start..rec_end];
        if rec_buf.len() < 2 {
            continue;
        }
        let key_len = u16::from_be_bytes([rec_buf[0], rec_buf[1]]) as usize;
        // After key: 2 (keyLength field) + key_len bytes
        let data_start = 2 + key_len;
        if data_start >= rec_buf.len() {
            continue;
        }
        let data_buf = &rec_buf[data_start..];

        // Check record type
        if data_buf.len() < 2 {
            continue;
        }
        let record_type = i16::from_be_bytes([data_buf[0], data_buf[1]]);
        if record_type == HFS_FILE_THREAD_RECORD {
            // Thread records point to the file's parent — not a file record, skip
            continue;
        }

        if let Some(file) = parse_file_record(data_buf) {
            // Only include if at least one extent block is free (deleted file candidate)
            let has_free_block = file
                .extents
                .iter()
                .any(|&(start, count)| has_any_free_block(alloc_bitmap, start, count));
            if has_free_block {
                candidates.push(file);
            }
        }
    }

    Ok(candidates)
}

/// Return true if any block in [start_block, start_block + count) is free.
fn has_any_free_block(alloc_bitmap: &[u8], start_block: u32, block_count: u32) -> bool {
    for i in 0..block_count {
        let block = start_block.saturating_add(i);
        if !is_block_allocated(alloc_bitmap, block) {
            return true;
        }
    }
    false
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

    /// Build a minimal HFS+ catalog file record with given CNID and data_size.
    fn make_file_record(cnid: u32, data_size: u64, mod_date: u32) -> Vec<u8> {
        let mut buf = vec![0u8; 256];
        // recordType = HFS_FILE_RECORD = 0x0200
        buf[0] = 0x02;
        buf[1] = 0x00;
        // flags at 2 (skip)
        // reserved at 4 (skip)
        // CNID at 8
        buf[8] = ((cnid >> 24) & 0xFF) as u8;
        buf[9] = ((cnid >> 16) & 0xFF) as u8;
        buf[10] = ((cnid >> 8) & 0xFF) as u8;
        buf[11] = (cnid & 0xFF) as u8;
        // contentModDate at 16
        buf[16] = ((mod_date >> 24) & 0xFF) as u8;
        buf[17] = ((mod_date >> 16) & 0xFF) as u8;
        buf[18] = ((mod_date >> 8) & 0xFF) as u8;
        buf[19] = (mod_date & 0xFF) as u8;
        // dataFork.logicalSize at 48
        buf[48] = ((data_size >> 56) & 0xFF) as u8;
        buf[49] = ((data_size >> 48) & 0xFF) as u8;
        buf[50] = ((data_size >> 40) & 0xFF) as u8;
        buf[51] = ((data_size >> 32) & 0xFF) as u8;
        buf[52] = ((data_size >> 24) & 0xFF) as u8;
        buf[53] = ((data_size >> 16) & 0xFF) as u8;
        buf[54] = ((data_size >> 8) & 0xFF) as u8;
        buf[55] = (data_size & 0xFF) as u8;
        // dataFork.extents[0] at 88: startBlock=10, blockCount=2
        buf[88] = 0x00;
        buf[89] = 0x00;
        buf[90] = 0x00;
        buf[91] = 0x0A; // start=10
        buf[92] = 0x00;
        buf[93] = 0x00;
        buf[94] = 0x00;
        buf[95] = 0x02; // count=2
        buf
    }

    #[test]
    fn test_parse_file_record() {
        let cases = vec![
            Case {
                name: "parse_file_record_valid",
                input: make_file_record(42, 4096, 0x7C25_B080),
                expected: Some((42u32, 4096u64)),
            },
            Case {
                name: "parse_file_record_too_small",
                input: vec![0u8; 10],
                expected: None,
            },
            Case {
                name: "parse_file_record_wrong_type",
                input: {
                    let mut buf = make_file_record(1, 0, 0);
                    buf[0] = 0x03; // HFS_FOLDER_RECORD
                    buf[1] = 0x00;
                    buf
                },
                expected: None,
            },
        ];

        for case in cases {
            let result = parse_file_record(&case.input);
            match (result, case.expected) {
                (Some(f), Some((cnid, size))) => {
                    assert_eq!(f.cnid, cnid, "FAILED case: {} — cnid", case.name);
                    assert_eq!(f.data_size, size, "FAILED case: {} — data_size", case.name);
                }
                (None, None) => {}
                (got, expected) => {
                    panic!(
                        "FAILED case: {} — got {:?}, expected {:?}",
                        case.name,
                        got.map(|f| f.cnid),
                        expected
                    );
                }
            }
        }
    }

    #[test]
    fn test_has_any_free_block() {
        let cases = vec![
            Case {
                name: "has_free_block_all_allocated",
                // bitmap: blocks 0-7 all allocated
                input: (vec![0xFFu8], 0u32, 8u32),
                expected: false,
            },
            Case {
                name: "has_free_block_some_free",
                // bitmap: only block 0 allocated, blocks 1-7 free
                input: (vec![0x80u8], 0u32, 4u32),
                expected: true,
            },
            Case {
                name: "has_free_block_empty_bitmap",
                input: (vec![], 0u32, 1u32),
                expected: true,
            },
            Case {
                name: "has_free_block_zero_count",
                input: (vec![0x00u8], 0u32, 0u32),
                expected: false,
            },
        ];

        for case in cases {
            let (bitmap, start, count) = case.input;
            let actual = has_any_free_block(&bitmap, start, count);
            assert_eq!(actual, case.expected, "FAILED case: {}", case.name);
        }
    }

    #[test]
    fn test_hfs_time_conversion() {
        use std::time::{Duration, UNIX_EPOCH};
        const HFS_TO_UNIX_OFFSET: u64 = 2_082_844_800;

        struct Input {
            hfs_time: u32,
        }
        struct Expected {
            is_some: bool,
            after_epoch: bool,
        }

        let cases = vec![
            (
                // 0xDA00_0000 = 3_657_433_088 HFS+ secs → unix 1_574_588_288 (Nov 2019)
                "hfs_time_conversion_valid",
                Input {
                    hfs_time: 0xDA00_0000,
                },
                Expected {
                    is_some: true,
                    after_epoch: true,
                },
            ),
            (
                "hfs_time_conversion_pre_epoch",
                Input { hfs_time: 0 },
                Expected {
                    is_some: false,
                    after_epoch: false,
                },
            ),
        ];

        for (name, input, expected) in cases {
            let result = u64::from(input.hfs_time).checked_sub(HFS_TO_UNIX_OFFSET);
            let system_time = result
                .and_then(|unix_secs| UNIX_EPOCH.checked_add(Duration::from_secs(unix_secs)));
            assert_eq!(
                system_time.is_some(),
                expected.is_some,
                "FAILED case: {name} — is_some"
            );
            if let Some(t) = system_time {
                assert_eq!(
                    t > UNIX_EPOCH,
                    expected.after_epoch,
                    "FAILED case: {name} — after_epoch"
                );
            }
        }
    }
}
