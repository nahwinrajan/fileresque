//! APFS space-manager free-space bitmap.
//!
//! Resolves the container's ephemeral space-manager object through the
//! checkpoint descriptor area, walks its chunk-info blocks, and answers
//! "is this block currently free?" against the on-disk allocation bitmap.
//!
//! ## Bitmap polarity
//!
//! The Apple File System Reference documents the per-chunk allocation bitmap as
//! one bit per block, but the *polarity* (whether a set bit means free or
//! allocated) is a detail that must be exactly right — a recovery tool that
//! inverts it would report reusable blocks as safe, or vice-versa. Rather than
//! hard-coding a guess, [`detect_polarity`] cross-checks both interpretations
//! against the authoritative `ci_free_count` for the chunk and uses whichever
//! matches. If neither matches (corruption, or an unexpected layout), the query
//! returns `None` ("unknown") so the probability engine stays conservative
//! instead of trusting a wrong answer.

use byteorder::{ReadBytesExt, LE};
use fileresque_core::error::AppError;
use std::io::Cursor;

use super::{reader::BlockReader, structs::NxSuperblock};

/// `o_type` field mask (low 16 bits select the object type).
const OBJ_TYPE_MASK: u32 = 0x0000_ffff;
/// `OBJECT_TYPE_SPACEMAN`.
const OBJECT_TYPE_SPACEMAN: u32 = 0x0000_0005;
/// `OBJECT_TYPE_CHECKPOINT_MAP`.
const OBJECT_TYPE_CHECKPOINT_MAP: u32 = 0x0000_000c;

/// Byte offset of `o_xid` within an `obj_phys_t`.
const OBJ_XID_OFFSET: u64 = 16;
/// Byte offset of `o_type` within an `obj_phys_t`.
const OBJ_TYPE_OFFSET: u64 = 24;

/// One managed run of contiguous blocks and its allocation bitmap.
#[derive(Debug, Clone)]
struct ChunkInfo {
    /// First block address covered by this chunk.
    addr: u64,
    /// Number of blocks in this chunk.
    block_count: u32,
    /// Authoritative free-block count for this chunk (`ci_free_count`).
    free_count: u32,
    /// Block address of this chunk's bitmap, or `0` for a uniform chunk
    /// (fully free or fully allocated, decided by `free_count`).
    bitmap_addr: u64,
}

/// Most-recently-read bitmap, cached so sequential block queries within one
/// chunk don't re-read the same block or re-detect polarity.
struct BitmapCache {
    bitmap_addr: u64,
    bytes: Vec<u8>,
    zero_is_free: bool,
}

/// The container's free-space map: an ordered set of chunks plus a one-entry
/// bitmap cache.
pub struct AllocationMap {
    chunks: Vec<ChunkInfo>,
    cache: Option<BitmapCache>,
}

impl AllocationMap {
    /// Resolve and load the container's space-manager free map.
    ///
    /// # Errors
    ///
    /// Returns [`AppError`] if the space manager cannot be located or its
    /// structures cannot be parsed.
    pub fn load(reader: &mut BlockReader, nx_sb: &NxSuperblock) -> Result<Self, AppError> {
        let sm_paddr = resolve_spaceman_paddr(reader, nx_sb)?;
        let sm_buf = reader.read_block(sm_paddr)?;

        // Confirm we landed on a spaceman object before trusting its layout.
        let (o_type, _) = read_obj_type_xid(&sm_buf)?;
        if o_type != OBJECT_TYPE_SPACEMAN {
            return Err(AppError::Internal(format!(
                "expected spaceman object, found type {o_type:#06x}"
            )));
        }

        let cib_addrs = collect_cib_addrs(reader, &sm_buf)?;

        let mut chunks = Vec::new();
        for cib_addr in cib_addrs {
            let cib_buf = reader.read_block(cib_addr)?;
            parse_cib_chunks(&cib_buf, &mut chunks)?;
        }
        chunks.sort_by_key(|c| c.addr);

        Ok(Self {
            chunks,
            cache: None,
        })
    }

    /// Whether `block` is currently free (not allocated to a live object).
    ///
    /// Returns `Ok(None)` when the block lies outside managed space or the
    /// bitmap polarity cannot be confirmed.
    ///
    /// # Errors
    ///
    /// Returns [`AppError`] if reading a bitmap block fails.
    pub fn is_free(
        &mut self,
        reader: &mut BlockReader,
        block: u64,
    ) -> Result<Option<bool>, AppError> {
        let Some(chunk) = self.find_chunk(block) else {
            return Ok(None);
        };

        if chunk.bitmap_addr == 0 {
            return Ok(uniform_chunk_free(&chunk));
        }

        let zero_is_free = self.ensure_cache(reader, &chunk)?;
        let Some(zero_is_free) = zero_is_free else {
            return Ok(None);
        };

        // SAFETY of unwrap avoided: cache is populated when zero_is_free is Some.
        let bit_index = block - chunk.addr;
        let Some(cache) = self.cache.as_ref() else {
            return Ok(None);
        };
        let set = bit_is_set(&cache.bytes, bit_index);
        Ok(Some(if zero_is_free { !set } else { set }))
    }

    /// Find the chunk containing `block`, if any.
    fn find_chunk(&self, block: u64) -> Option<ChunkInfo> {
        let idx = self
            .chunks
            .partition_point(|c| c.addr <= block)
            .checked_sub(1)?;
        let chunk = self.chunks.get(idx)?;
        let end = chunk.addr + u64::from(chunk.block_count);
        if block >= chunk.addr && block < end {
            Some(chunk.clone())
        } else {
            None
        }
    }

    /// Ensure the cache holds `chunk`'s bitmap; return its `zero_is_free`
    /// polarity, or `None` if polarity cannot be determined.
    fn ensure_cache(
        &mut self,
        reader: &mut BlockReader,
        chunk: &ChunkInfo,
    ) -> Result<Option<bool>, AppError> {
        if let Some(cache) = &self.cache {
            if cache.bitmap_addr == chunk.bitmap_addr {
                return Ok(Some(cache.zero_is_free));
            }
        }

        let bytes = reader.read_block(chunk.bitmap_addr)?;
        let Some(zero_is_free) = detect_polarity(&bytes, chunk.block_count, chunk.free_count)
        else {
            return Ok(None);
        };
        self.cache = Some(BitmapCache {
            bitmap_addr: chunk.bitmap_addr,
            bytes,
            zero_is_free,
        });
        Ok(Some(zero_is_free))
    }
}

/// Free state of a chunk with no bitmap: uniform free or uniform allocated.
fn uniform_chunk_free(chunk: &ChunkInfo) -> Option<bool> {
    if chunk.free_count == chunk.block_count {
        Some(true)
    } else if chunk.free_count == 0 {
        Some(false)
    } else {
        None
    }
}

/// Test bit `index` (LSB-first within each byte).
fn bit_is_set(bytes: &[u8], index: u64) -> bool {
    let byte = (index / 8) as usize;
    let bit = (index % 8) as u8;
    bytes.get(byte).is_some_and(|b| (b >> bit) & 1 == 1)
}

/// Determine whether a clear bit means "free" by matching the counted free
/// bits against the authoritative `free_count`. Returns `None` if neither
/// polarity matches.
fn detect_polarity(bytes: &[u8], block_count: u32, free_count: u32) -> Option<bool> {
    let mut ones: u32 = 0;
    for i in 0..u64::from(block_count) {
        if bit_is_set(bytes, i) {
            ones += 1;
        }
    }
    let zeros = block_count - ones;
    if zeros == free_count {
        Some(true) // clear bit == free
    } else if ones == free_count {
        Some(false) // set bit == free
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Spaceman resolution + parsing
// ---------------------------------------------------------------------------

/// Read `o_type` (masked) and `o_xid` from an object's `obj_phys_t` header.
fn read_obj_type_xid(buf: &[u8]) -> Result<(u32, u64), AppError> {
    let mut cur = Cursor::new(buf);
    cur.set_position(OBJ_XID_OFFSET);
    let xid = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    cur.set_position(OBJ_TYPE_OFFSET);
    let o_type = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((o_type & OBJ_TYPE_MASK, xid))
}

/// Scan the checkpoint descriptor area for the newest mapping of the
/// space-manager ephemeral OID, returning its physical address.
fn resolve_spaceman_paddr(reader: &mut BlockReader, nx_sb: &NxSuperblock) -> Result<u64, AppError> {
    let target = nx_sb.spaceman_oid;
    let mut best: Option<(u64, u64)> = None; // (xid, paddr)

    for i in 0..u64::from(nx_sb.xp_desc_len) {
        let block = nx_sb.xp_desc_base + i;
        let Ok(buf) = reader.read_block(block) else {
            continue;
        };
        let Ok((o_type, _)) = read_obj_type_xid(&buf) else {
            continue;
        };
        if o_type != OBJECT_TYPE_CHECKPOINT_MAP {
            continue;
        }
        if let Some((xid, paddr)) = checkpoint_map_lookup(&buf, target) {
            if best.is_none_or(|(best_xid, _)| xid > best_xid) {
                best = Some((xid, paddr));
            }
        }
    }

    best.map(|(_, paddr)| paddr).ok_or_else(|| {
        AppError::Internal("space-manager object not found in checkpoint area".to_string())
    })
}

/// Search a `checkpoint_map_phys` block for `target_oid`, returning
/// `(checkpoint_xid, paddr)` of the mapping when present.
fn checkpoint_map_lookup(buf: &[u8], target_oid: u64) -> Option<(u64, u64)> {
    const MAP_START: u64 = 40;
    const MAP_SIZE: u64 = 40;
    const CPM_OID_OFF: u64 = 16;
    const CPM_PADDR_OFF: u64 = 24;

    let (_, xid) = read_obj_type_xid(buf).ok()?;
    let mut cur = Cursor::new(buf);
    cur.set_position(32); // skip obj_phys_t
    let _flags = cur.read_u32::<LE>().ok()?;
    let count = cur.read_u32::<LE>().ok()?;

    for i in 0..u64::from(count) {
        let base = MAP_START + i * MAP_SIZE;
        cur.set_position(base + CPM_OID_OFF);
        let oid = cur.read_u64::<LE>().ok()?;
        if oid == target_oid {
            cur.set_position(base + CPM_PADDR_OFF);
            let paddr = cur.read_u64::<LE>().ok()?;
            return Some((xid, paddr));
        }
    }
    None
}

/// From a `spaceman_phys` block, collect the chunk-info-block (cib) addresses
/// for the main device, following the chunk-info-address-block (cab) layer when
/// present.
// `cib` (chunk-info block) and `cab` (chunk-info address block) are distinct
// APFS structures; their spec-accurate names are intentionally similar.
#[allow(clippy::similar_names)]
fn collect_cib_addrs(reader: &mut BlockReader, sm_buf: &[u8]) -> Result<Vec<u64>, AppError> {
    // spaceman_device_t sm_dev[SD_MAIN] starts at offset 64 within spaceman_phys.
    const SM_DEV_MAIN: u64 = 64;
    const CIB_COUNT_OFF: u64 = SM_DEV_MAIN + 16;
    const CAB_COUNT_OFF: u64 = SM_DEV_MAIN + 20;
    const ADDR_OFFSET_OFF: u64 = SM_DEV_MAIN + 32;

    let mut cur = Cursor::new(sm_buf);
    cur.set_position(CIB_COUNT_OFF);
    let cib_count = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    cur.set_position(CAB_COUNT_OFF);
    let cab_count = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    cur.set_position(ADDR_OFFSET_OFF);
    let addr_offset = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let addrs = read_u64_array(
        sm_buf,
        u64::from(addr_offset),
        addr_count(cib_count, cab_count),
    )?;

    if cab_count == 0 {
        return Ok(addrs);
    }

    // Indirect: each address is a cab whose entries are cib addresses.
    let mut cib_addrs = Vec::new();
    for cab_addr in addrs {
        let cab_buf = reader.read_block(cab_addr)?;
        parse_cab_cib_addrs(&cab_buf, &mut cib_addrs)?;
    }
    Ok(cib_addrs)
}

/// Number of entries in the spaceman address array.
// cib/cab are spec-accurate APFS acronyms; their similarity is intentional.
#[allow(clippy::similar_names)]
fn addr_count(cib_count: u32, cab_count: u32) -> u32 {
    if cab_count == 0 {
        cib_count
    } else {
        cab_count
    }
}

/// Read `count` little-endian u64s starting at byte `offset` within `buf`.
fn read_u64_array(buf: &[u8], offset: u64, count: u32) -> Result<Vec<u64>, AppError> {
    let mut cur = Cursor::new(buf);
    cur.set_position(offset);
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let v = cur
            .read_u64::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        out.push(v);
    }
    Ok(out)
}

/// Append the cib addresses listed in a `chunk_info_address_block` (cab).
fn parse_cab_cib_addrs(cab_buf: &[u8], out: &mut Vec<u64>) -> Result<(), AppError> {
    // obj_phys_t(32) + cab_index(4) + cab_cib_count(4), then paddr[] at 40.
    let mut cur = Cursor::new(cab_buf);
    cur.set_position(36);
    let count = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let addrs = read_u64_array(cab_buf, 40, count)?;
    out.extend(addrs);
    Ok(())
}

/// Append the chunks described by a `chunk_info_block` (cib).
fn parse_cib_chunks(cib_buf: &[u8], out: &mut Vec<ChunkInfo>) -> Result<(), AppError> {
    // obj_phys_t(32) + cib_index(4) + cib_chunk_info_count(4), then chunk_info[] at 40.
    const CHUNKS_START: u64 = 40;
    const CHUNK_SIZE: u64 = 32;
    const CI_ADDR_OFF: u64 = 8;
    // ci_block_count(4) then ci_free_count(4) are read sequentially from here.
    const CI_BLOCK_COUNT_OFF: u64 = 16;
    const CI_BITMAP_ADDR_OFF: u64 = 24;

    let mut cur = Cursor::new(cib_buf);
    cur.set_position(36);
    let count = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    for i in 0..u64::from(count) {
        let base = CHUNKS_START + i * CHUNK_SIZE;
        cur.set_position(base + CI_ADDR_OFF);
        let addr = cur
            .read_u64::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        cur.set_position(base + CI_BLOCK_COUNT_OFF);
        let block_count = cur
            .read_u32::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let free_count = cur
            .read_u32::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        cur.set_position(base + CI_BITMAP_ADDR_OFF);
        let bitmap_addr = cur
            .read_u64::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        out.push(ChunkInfo {
            addr,
            block_count,
            free_count,
            bitmap_addr,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_is_set_reads_lsb_first() {
        // byte 0 = 0b0000_0101 → bits 0 and 2 set
        let bytes = [0b0000_0101u8, 0b1000_0000u8];
        assert!(bit_is_set(&bytes, 0));
        assert!(!bit_is_set(&bytes, 1));
        assert!(bit_is_set(&bytes, 2));
        assert!(bit_is_set(&bytes, 15)); // byte 1, bit 7
        assert!(!bit_is_set(&bytes, 99)); // out of range → false
    }

    #[test]
    fn detect_polarity_matches_free_count() {
        // 8 blocks, bitmap 0b0000_0011 → 2 ones, 6 zeros.
        let bytes = [0b0000_0011u8];
        // free_count == zeros(6) → clear bit means free
        assert_eq!(detect_polarity(&bytes, 8, 6), Some(true));
        // free_count == ones(2) → set bit means free
        assert_eq!(detect_polarity(&bytes, 8, 2), Some(false));
        // free_count matches neither → unknown
        assert_eq!(detect_polarity(&bytes, 8, 4), None);
    }

    #[test]
    fn uniform_chunk_free_branches() {
        let full_free = ChunkInfo {
            addr: 0,
            block_count: 10,
            free_count: 10,
            bitmap_addr: 0,
        };
        let full_used = ChunkInfo {
            addr: 0,
            block_count: 10,
            free_count: 0,
            bitmap_addr: 0,
        };
        let mixed = ChunkInfo {
            addr: 0,
            block_count: 10,
            free_count: 5,
            bitmap_addr: 0,
        };
        assert_eq!(uniform_chunk_free(&full_free), Some(true));
        assert_eq!(uniform_chunk_free(&full_used), Some(false));
        assert_eq!(uniform_chunk_free(&mixed), None);
    }

    #[test]
    fn find_chunk_locates_owning_run() {
        let map = AllocationMap {
            chunks: vec![
                ChunkInfo {
                    addr: 100,
                    block_count: 50,
                    free_count: 0,
                    bitmap_addr: 0,
                },
                ChunkInfo {
                    addr: 200,
                    block_count: 50,
                    free_count: 0,
                    bitmap_addr: 0,
                },
            ],
            cache: None,
        };
        assert_eq!(map.find_chunk(120).map(|c| c.addr), Some(100));
        assert_eq!(map.find_chunk(249).map(|c| c.addr), Some(200));
        assert!(map.find_chunk(150).is_none()); // gap between chunks
        assert!(map.find_chunk(50).is_none()); // before first chunk
        assert!(map.find_chunk(300).is_none()); // after last chunk
    }

    #[test]
    fn parse_cib_chunks_reads_entries() {
        // One cib with a single chunk_info entry.
        let mut buf = vec![0u8; 512];
        // cib_chunk_info_count at offset 36 = 1
        buf[36..40].copy_from_slice(&1u32.to_le_bytes());
        // chunk_info[0] at offset 40: ci_xid(8) ci_addr(8) ci_block_count(4)
        // ci_free_count(4) ci_bitmap_addr(8)
        buf[48..56].copy_from_slice(&1234u64.to_le_bytes()); // ci_addr
        buf[56..60].copy_from_slice(&64u32.to_le_bytes()); // ci_block_count
        buf[60..64].copy_from_slice(&60u32.to_le_bytes()); // ci_free_count
        buf[64..72].copy_from_slice(&9999u64.to_le_bytes()); // ci_bitmap_addr

        let mut chunks = Vec::new();
        // JUSTIFIED: test-only; synthetic buffer is well-formed
        parse_cib_chunks(&buf, &mut chunks).expect("parse must succeed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].addr, 1234);
        assert_eq!(chunks[0].block_count, 64);
        assert_eq!(chunks[0].free_count, 60);
        assert_eq!(chunks[0].bitmap_addr, 9999);
    }

    #[test]
    fn checkpoint_map_lookup_finds_target() {
        let mut buf = vec![0u8; 512];
        // o_xid at offset 16
        buf[16..24].copy_from_slice(&7u64.to_le_bytes());
        // cpm_count at offset 36 = 2
        buf[36..40].copy_from_slice(&2u32.to_le_bytes());
        // mapping[0] at 40: cpm_oid at +16, cpm_paddr at +24
        buf[40 + 16..40 + 24].copy_from_slice(&111u64.to_le_bytes());
        buf[40 + 24..40 + 32].copy_from_slice(&500u64.to_le_bytes());
        // mapping[1] at 80
        buf[80 + 16..80 + 24].copy_from_slice(&222u64.to_le_bytes());
        buf[80 + 24..80 + 32].copy_from_slice(&600u64.to_le_bytes());

        assert_eq!(checkpoint_map_lookup(&buf, 222), Some((7, 600)));
        assert_eq!(checkpoint_map_lookup(&buf, 999), None);
    }

    #[test]
    fn addr_count_selects_layer() {
        assert_eq!(addr_count(5, 0), 5); // direct cib
        assert_eq!(addr_count(5, 2), 2); // via cab
    }
}
