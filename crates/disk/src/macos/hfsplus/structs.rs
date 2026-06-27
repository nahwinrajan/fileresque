use byteorder::{ReadBytesExt, BE};
use fileresque_core::error::AppError;
use std::io::{Cursor, Seek, SeekFrom};

/// HFS+ Volume Header magic: "H+" = 0x482B
pub(crate) const HFS_PLUS_MAGIC: u16 = 0x482B;
/// HFSX (case-sensitive HFS+) magic: "HX" = 0x4858
pub(crate) const HFSX_MAGIC: u16 = 0x4858;

/// HFS+ catalog record types (stored in leaf node records)
pub(crate) const HFS_FILE_RECORD: i16 = 0x0200;
pub(crate) const HFS_FILE_THREAD_RECORD: i16 = 0x0400;

/// B*-tree node descriptor kinds
pub(crate) const BT_LEAF_NODE: i8 = -1;

/// Minimum valid size of a Volume Header buffer (we need at least 304 bytes
/// to read through the catalogFile extents at offset 224+80=304).
pub(crate) const MIN_VOLUME_HEADER_SIZE: usize = 512;

/// Number of extent descriptors stored in the fork data structure
const EXTENT_COUNT: usize = 8;

/// Parsed subset of the HFS+ Volume Header (512 bytes at disk offset 1024).
/// All integer fields are stored big-endian on disk.
#[derive(Debug, Clone)]
pub(crate) struct HfsPlusVolumeHeader {
    pub(crate) block_size: u32,
    pub(crate) total_blocks: u32,
    pub(crate) free_blocks: u32,
    /// First 8 extents of the Allocation Bitmap File: (startBlock, blockCount)
    pub(crate) alloc_extents: Vec<(u32, u32)>,
    /// First 8 extents of the Catalog B*-tree File: (startBlock, blockCount)
    pub(crate) catalog_extents: Vec<(u32, u32)>,
    /// Logical size in bytes of the catalog file
    pub(crate) catalog_logical_size: u64,
}

/// A deleted file candidate found while walking catalog B*-tree leaf nodes.
#[derive(Debug)]
pub(crate) struct CatalogDeletedFile {
    /// Catalog Node ID (unique per file/folder in HFS+)
    pub(crate) cnid: u32,
    pub(crate) name: Option<String>,
    /// Logical data fork size in bytes
    pub(crate) data_size: u64,
    /// Last-modified time in HFS+ epoch (seconds since 1904-01-01 00:00 UTC)
    pub(crate) mod_date: Option<u32>,
    /// Up to 8 data fork extents: (startBlock, blockCount)
    pub(crate) extents: Vec<(u32, u32)>,
}

/// Read `count` extent descriptors from `cur`, each 8 bytes: startBlock(4) + blockCount(4).
fn read_extents_from_cursor(
    cur: &mut Cursor<&[u8]>,
    count: usize,
) -> Result<Vec<(u32, u32)>, AppError> {
    let mut extents = Vec::with_capacity(count);
    for _ in 0..count {
        let start = cur
            .read_u32::<BE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let count_blocks = cur
            .read_u32::<BE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if start != 0 || count_blocks != 0 {
            extents.push((start, count_blocks));
        }
    }
    Ok(extents)
}

/// Parse the HFS+ Volume Header from a 512-byte (or larger) buffer.
/// The buffer must begin at byte offset 1024 of the raw device
/// (i.e. the caller reads 512 bytes starting at device offset 1024).
pub(crate) fn parse_volume_header(buf: &[u8]) -> Result<HfsPlusVolumeHeader, AppError> {
    if buf.len() < MIN_VOLUME_HEADER_SIZE {
        return Err(AppError::Internal(format!(
            "HFS+ volume header buffer too small: {} bytes (need {})",
            buf.len(),
            MIN_VOLUME_HEADER_SIZE
        )));
    }

    let mut cur = Cursor::new(buf);

    // offset 0: signature
    let sig = cur
        .read_u16::<BE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if sig != HFS_PLUS_MAGIC && sig != HFSX_MAGIC {
        return Err(AppError::Internal(format!(
            "HFS+ magic mismatch: {sig:#06x} (expected {HFS_PLUS_MAGIC:#06x} or {HFSX_MAGIC:#06x})"
        )));
    }

    // offset 2: version (skip)
    // offset 4: attributes (skip)
    // offset 8: lastMountedVersion (skip)
    // offset 12: journalInfoBlock (skip)
    // offset 16-39: dates (skip)
    // offset 32: fileCount (skip)
    // offset 36: folderCount (skip)
    // offset 40: blockSize
    cur.seek(SeekFrom::Start(40))
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let block_size = cur
        .read_u32::<BE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // offset 44: totalBlocks
    let total_blocks = cur
        .read_u32::<BE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // offset 48: freeBlocks
    let free_blocks = cur
        .read_u32::<BE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // offset 52-111: nextAllocation(4), rsrcClumpSize(4), dataClumpSize(4),
    //                nextCatalogID(4), writeCount(4), encodingsBitmap(8),
    //                finderInfo(32) — total 60 bytes, skip to offset 112
    cur.seek(SeekFrom::Start(112))
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // offset 112: allocationFile.logicalSize (8) — skip
    // offset 120: allocationFile.clumpSize (4) — skip
    // offset 124: allocationFile.totalBlocks (4) — skip
    // offset 128: allocationFile.extents (8 × 8 = 64 bytes)
    cur.seek(SeekFrom::Start(128))
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let alloc_extents = read_extents_from_cursor(&mut cur, EXTENT_COUNT)?;

    // offset 208: catalogFile.logicalSize (8)
    cur.seek(SeekFrom::Start(208))
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let catalog_logical_size = cur
        .read_u64::<BE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // offset 216: catalogFile.clumpSize (4) — skip
    // offset 220: catalogFile.totalBlocks (4) — skip
    // offset 224: catalogFile.extents (8 × 8 = 64 bytes)
    cur.seek(SeekFrom::Start(224))
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let catalog_extents = read_extents_from_cursor(&mut cur, EXTENT_COUNT)?;

    Ok(HfsPlusVolumeHeader {
        block_size,
        total_blocks,
        free_blocks,
        alloc_extents,
        catalog_extents,
        catalog_logical_size,
    })
}

/// Return `true` if `block_num` is marked allocated in the HFS+ allocation bitmap.
/// HFS+ allocation bitmap is MSB-first: bit 7 of byte 0 covers block 0.
/// Out-of-range block numbers are treated as free (return `false`).
pub(crate) fn is_block_allocated(alloc_bitmap: &[u8], block_num: u32) -> bool {
    let byte_idx = (block_num / 8) as usize;
    let bit_shift = 7 - (block_num % 8);
    if byte_idx >= alloc_bitmap.len() {
        return false;
    }
    (alloc_bitmap[byte_idx] >> bit_shift) & 1 == 1
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

    /// Build a minimal 512-byte buffer with chosen magic and optional blockSize.
    fn make_header_buf(magic: u16, block_size: Option<u32>) -> Vec<u8> {
        let mut buf = vec![0u8; 512];
        buf[0] = (magic >> 8) as u8;
        buf[1] = (magic & 0xFF) as u8;
        if let Some(bs) = block_size {
            buf[40] = ((bs >> 24) & 0xFF) as u8;
            buf[41] = ((bs >> 16) & 0xFF) as u8;
            buf[42] = ((bs >> 8) & 0xFF) as u8;
            buf[43] = (bs & 0xFF) as u8;
        }
        buf
    }

    #[test]
    fn test_parse_volume_header() {
        struct Input {
            buf: Vec<u8>,
        }
        struct Expected {
            is_ok: bool,
            block_size: Option<u32>,
        }

        let cases = vec![
            (
                "parse_volume_header_valid",
                Input {
                    buf: make_header_buf(HFS_PLUS_MAGIC, Some(4096)),
                },
                Expected {
                    is_ok: true,
                    block_size: Some(4096),
                },
            ),
            (
                "parse_volume_header_hfsx_magic",
                Input {
                    buf: make_header_buf(HFSX_MAGIC, Some(512)),
                },
                Expected {
                    is_ok: true,
                    block_size: Some(512),
                },
            ),
            (
                "parse_volume_header_wrong_magic",
                Input {
                    buf: make_header_buf(0xFFFF, None),
                },
                Expected {
                    is_ok: false,
                    block_size: None,
                },
            ),
            (
                "parse_volume_header_too_small",
                Input { buf: vec![0u8; 100] },
                Expected {
                    is_ok: false,
                    block_size: None,
                },
            ),
        ];

        for (name, input, expected) in cases {
            let result = parse_volume_header(&input.buf);
            assert_eq!(
                result.is_ok(),
                expected.is_ok,
                "FAILED case: {name} — ok mismatch"
            );
            if let (Ok(hdr), Some(bs)) = (result, expected.block_size) {
                assert_eq!(hdr.block_size, bs, "FAILED case: {name} — block_size mismatch");
            }
        }
    }

    #[test]
    fn test_is_block_allocated() {
        let cases = vec![
            Case {
                name: "is_block_allocated_set_msb",
                input: (vec![0x80u8], 0u32),
                expected: true,
            },
            Case {
                name: "is_block_allocated_clear_second_bit",
                input: (vec![0x80u8], 1u32),
                expected: false,
            },
            Case {
                name: "is_block_allocated_second_byte",
                input: (vec![0x00u8, 0x01u8], 15u32),
                expected: true,
            },
            Case {
                name: "is_block_allocated_out_of_range",
                input: (vec![], 0u32),
                expected: false,
            },
            Case {
                name: "is_block_allocated_all_set",
                input: (vec![0xFFu8], 7u32),
                expected: true,
            },
        ];

        for case in cases {
            let (bitmap, block) = case.input;
            let actual = is_block_allocated(&bitmap, block);
            assert_eq!(actual, case.expected, "FAILED case: {}", case.name);
        }
    }
}
