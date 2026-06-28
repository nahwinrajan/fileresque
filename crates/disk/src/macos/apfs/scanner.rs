use byteorder::{LE, ReadBytesExt};
use fileresque_core::{
    error::AppError,
    types::{DeletedFileEntry, FileSystem},
};
use std::io::Cursor;
use tokio::sync::mpsc;

use super::{
    reader::BlockReader,
    structs::{
        APFS_MAGIC, APFS_TYPE_INODE, MAX_BLOCK_SIZE, MIN_BLOCK_SIZE, NX_MAGIC,
        NX_SUPERBLOCK_MAGIC_OFFSET, ApfsSuperblock, InodeRecord, NxSuperblock,
    },
};

// ---------------------------------------------------------------------------
// NX (container) superblock
// ---------------------------------------------------------------------------

/// Parse a container superblock from a raw block buffer.
///
/// Offsets are taken from the Apple APFS Reference (2019), Table 5-2.
/// The `obj_phys_t` header occupies the first 32 bytes.
///
/// # Errors
///
/// Returns `Err` on buffer too small, magic mismatch, or invalid field values.
pub fn parse_nx_superblock(buf: &[u8]) -> Result<NxSuperblock, AppError> {
    if buf.len() < 1024 {
        return Err(AppError::Internal(
            "Buffer too small for NX superblock".to_string(),
        ));
    }

    let mut cur = Cursor::new(buf);
    validate_nx_magic(&mut cur)?;

    let block_size = read_nx_block_size(&mut cur)?;
    let block_count = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Skip: nx_features(8) + nx_readonly_compatible_features(8)
    //       + nx_incompatible_features(8) + nx_uuid(16)
    //       + nx_next_oid(8) + nx_next_xid(8) = 56 bytes
    advance(&mut cur, 56)?;

    let (xp_desc_base, xp_desc_len) = read_checkpoint_info(&mut cur)?;

    let omap_oid = read_nx_omap_oid(&mut cur)?;

    let (max_fs, fs_oids) = read_nx_fs_oids(&mut cur)?;
    let _ = max_fs; // used internally only

    Ok(NxSuperblock {
        block_size,
        block_count,
        omap_oid,
        fs_oids,
        xp_desc_base,
        xp_desc_len,
    })
}

fn validate_nx_magic(cur: &mut Cursor<&[u8]>) -> Result<(), AppError> {
    cur.set_position(NX_SUPERBLOCK_MAGIC_OFFSET);
    let magic = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if magic != NX_MAGIC {
        return Err(AppError::Internal(format!(
            "NX superblock magic mismatch: {magic:#010x} (expected {NX_MAGIC:#010x})"
        )));
    }
    Ok(())
}

fn read_nx_block_size(cur: &mut Cursor<&[u8]>) -> Result<u32, AppError> {
    let block_size = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if !(MIN_BLOCK_SIZE..=MAX_BLOCK_SIZE).contains(&block_size) || block_size % 512 != 0 {
        return Err(AppError::Internal(format!(
            "Invalid block_size: {block_size} (must be 512-65536, multiple of 512)"
        )));
    }
    Ok(block_size)
}

/// Read checkpoint descriptor base + length.
///
/// After `nx_block_count` (u64) comes:
///   nx_features(8) + nx_readonly_compatible_features(8)
///   + nx_incompatible_features(8) + nx_uuid(16)
///   + nx_next_oid(8) + nx_next_xid(8) = 56 bytes   [caller already skipped]
///
/// Then:
///   nx_xp_desc_blocks(4) + nx_xp_data_blocks(4) + nx_xp_desc_base(8i)
///   + nx_xp_data_base(8) + nx_xp_desc_next(4) + nx_xp_data_next(4)
///   + nx_xp_desc_index(4) + nx_xp_desc_len(4)
fn read_checkpoint_info(cur: &mut Cursor<&[u8]>) -> Result<(u64, u32), AppError> {
    let xp_desc_blocks = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let _xp_data_blocks = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let xp_desc_base_raw = cur
        .read_i64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let xp_desc_base = u64::try_from(xp_desc_base_raw)
        .map_err(|_| AppError::Internal("Negative xp_desc_base".to_string()))?;

    // nx_xp_data_base(8) + nx_xp_desc_next(4) + nx_xp_data_next(4)
    // + nx_xp_desc_index(4)
    advance(cur, 20)?;

    let xp_desc_len = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((xp_desc_base, xp_desc_len.min(xp_desc_blocks)))
}

/// Read spaceman, omap, and reaper OIDs; return only omap_oid.
///
/// After `xp_desc_len` comes:
///   nx_xp_data_len(4) + nx_spaceman_oid(8) + nx_omap_oid(8)
fn read_nx_omap_oid(cur: &mut Cursor<&[u8]>) -> Result<u64, AppError> {
    // nx_xp_data_len
    advance(cur, 4)?;
    // nx_spaceman_oid
    let _spaceman_oid = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let omap_oid = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    // nx_reaper_oid — skip
    advance(cur, 8)?;
    Ok(omap_oid)
}

/// Read `nx_test_type`, `nx_max_file_systems`, and `nx_fs_oid[]`.
fn read_nx_fs_oids(cur: &mut Cursor<&[u8]>) -> Result<(usize, Vec<u64>), AppError> {
    // nx_test_type
    let _test_type = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let max_fs_raw = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let max_fs = (max_fs_raw as usize).min(100);

    let mut fs_oids = Vec::with_capacity(max_fs);
    for _ in 0..max_fs {
        let oid = cur
            .read_u64::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if oid != 0 {
            fs_oids.push(oid);
        }
    }
    Ok((max_fs, fs_oids))
}

// ---------------------------------------------------------------------------
// APFS volume superblock
// ---------------------------------------------------------------------------

/// Parse an APFS volume superblock from a raw block buffer.
///
/// # Errors
///
/// Returns `Err` on buffer too small or magic mismatch.
pub(crate) fn parse_apfs_superblock(buf: &[u8]) -> Result<ApfsSuperblock, AppError> {
    if buf.len() < 512 {
        return Err(AppError::Internal(
            "Buffer too small for APFS superblock".to_string(),
        ));
    }

    let mut cur = Cursor::new(buf);
    // obj_phys_t header: 32 bytes
    cur.set_position(32);
    let magic = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if magic != APFS_MAGIC {
        return Err(AppError::Internal(format!(
            "APFS volume magic mismatch: {magic:#010x} (expected {APFS_MAGIC:#010x})"
        )));
    }

    // apfs_fs_index(4) + apfs_features(8) + apfs_readonly_compatible_features(8)
    // + apfs_incompatible_features(8) + apfs_unmount_time(8)
    // + apfs_fs_reserve_block_count(8) + apfs_quota_block_count(8)
    // + apfs_fs_alloc_count(8) + apfs_meta_crypto(20) + apfs_root_tree_type(4)
    // + apfs_extentref_tree_type(4) + apfs_snap_meta_tree_type(4)
    // + apfs_omap_oid(8)
    // total skip after magic(4): 4+8+8+8+8+8+8+8+20+4+4+4 = 92, then omap_oid(8)
    advance(&mut cur, 92)?;

    let omap_oid = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // apfs_root_tree_oid(8)
    let root_tree_oid = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // apfs_extentref_tree_oid(8) + apfs_snap_meta_tree_oid(8)
    // + apfs_revert_to_xid(8) + apfs_revert_to_sblock_oid(8)
    // + apfs_next_obj_id(8) + apfs_num_files(8) + apfs_num_directories(8)
    // + apfs_num_symlinks(8) + apfs_num_other_fsobjects(8) + apfs_num_snapshots(8)
    // + apfs_total_blocks_alloced(8) + apfs_total_blocks_freed(8)
    // + apfs_vol_uuid(16) + apfs_last_mod_time(8) + apfs_fs_flags(8)
    // + apfs_formatted_by(32) + apfs_modified_by(8*32=256) = 8+8+8+8+8+8+8+8+8+8+8+8+16+8+8+32+256 = 430
    advance(&mut cur, 430)?;

    // apfs_volname: 256 bytes, null-terminated UTF-8
    let mut name_buf = vec![0u8; 256];
    {
        let pos = usize::try_from(cur.position())
            .map_err(|_| AppError::Internal("cursor position overflow".to_string()))?;
        let available = buf.len().saturating_sub(pos);
        let copy_len = name_buf.len().min(available);
        name_buf[..copy_len].copy_from_slice(&buf[pos..pos + copy_len]);
    }
    let volume_name = parse_cstring(&name_buf);

    Ok(ApfsSuperblock {
        volume_name,
        omap_oid,
        root_tree_oid,
    })
}

// ---------------------------------------------------------------------------
// OMap B-tree lookup
// ---------------------------------------------------------------------------

/// Resolve an object ID to a physical block address using the OMap B-tree.
///
/// The OMap (`apfs_omap_phys_t`) starts with `obj_phys_t` (32 bytes) then:
///   om_flags(4) + om_snap_count(4) + om_tree_type(4) + om_snapshot_tree_type(4)
///   + om_tree_oid(8) + om_snapshot_list_oid(8) + om_most_recent_snap(8)
///   + om_pending_revert_min(8) + om_pending_revert_max(8)
///
/// `om_tree_oid` at offset 56 is the B-tree root node OID. For simplicity we
/// treat that OID as a *physical* block address (valid when it is a physical
/// object — most container OMap objects are physical).
///
/// # Errors
///
/// Returns `Err(AppError::Internal)` when the OID is not found.
pub(crate) fn omap_lookup(
    reader: &mut BlockReader,
    omap_phys_block: u64,
    target_oid: u64,
) -> Result<u64, AppError> {
    let omap_buf = reader.read_block(omap_phys_block)?;
    if omap_buf.len() < 64 {
        return Err(AppError::Internal("OMap block too small".to_string()));
    }
    let tree_oid = {
        let mut cur = Cursor::new(&omap_buf[..]);
        // obj_phys_t(32) + om_flags(4) + om_snap_count(4)
        // + om_tree_type(4) + om_snapshot_tree_type(4) = 48 bytes before om_tree_oid
        cur.set_position(48);
        cur.read_u64::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?
    };

    // Walk B-tree nodes starting from tree_oid (treated as physical block addr)
    omap_btree_lookup(reader, tree_oid, target_oid)
}

/// Traverse an OMap B-tree from `node_block` looking for `target_oid`.
///
/// Key  = { oid: u64, xid: u64 }  (16 bytes, fixed)
/// Value = { flags: u32, size: u32, paddr: i64 }  (16 bytes, fixed)
fn omap_btree_lookup(
    reader: &mut BlockReader,
    node_block: u64,
    target_oid: u64,
) -> Result<u64, AppError> {
    let buf = reader.read_block(node_block)?;
    let (level, nkeys) = read_btn_header(&buf)?;

    if level == 0 {
        // Leaf node — linear scan for target_oid with highest xid
        omap_leaf_scan(&buf, nkeys, target_oid)
    } else {
        // Internal node — find the correct child pointer and recurse
        omap_internal_lookup(reader, &buf, nkeys, target_oid)
    }
}

/// Parse `btn_level` and `btn_nkeys` from a B-tree node header.
///
/// B-tree node layout (from APFS Reference §11.1):
///   btn_o (obj_phys_t): 32 bytes
///   btn_flags: u16
///   btn_level: u16
///   btn_nkeys: u32
fn read_btn_header(buf: &[u8]) -> Result<(u16, u32), AppError> {
    if buf.len() < 40 {
        return Err(AppError::Internal(
            "B-tree node buffer too small".to_string(),
        ));
    }
    let mut cur = Cursor::new(buf);
    // Skip obj_phys_t (32 bytes) + btn_flags (2 bytes)
    cur.set_position(34);
    let level = cur
        .read_u16::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let nkeys = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((level, nkeys))
}

/// Scan OMap leaf node entries for `target_oid`, returning the block address
/// from the entry with the highest transaction ID (xid).
fn omap_leaf_scan(buf: &[u8], nkeys: u32, target_oid: u64) -> Result<u64, AppError> {
    // Fixed-size OMap keys and values: 16 bytes each.
    // The key+value area starts at offset 72 (after the node header).
    // Layout: keys[0..nkeys] then values[0..nkeys] packed contiguously.
    const HDR_SIZE: usize = 72;
    const KV_SIZE: usize = 16;

    let nkeys = nkeys as usize;
    let keys_start = HDR_SIZE;
    let vals_start = HDR_SIZE + nkeys * KV_SIZE;
    let required = vals_start + nkeys * KV_SIZE;

    if buf.len() < required {
        return Err(AppError::Internal(
            "OMap leaf node buffer too small for claimed nkeys".to_string(),
        ));
    }

    let mut best_xid: u64 = 0;
    let mut best_paddr: Option<u64> = None;

    for i in 0..nkeys {
        let key_off = keys_start + i * KV_SIZE;
        let val_off = vals_start + i * KV_SIZE;

        let mut kc = Cursor::new(&buf[key_off..key_off + KV_SIZE]);
        let oid = kc
            .read_u64::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let xid = kc
            .read_u64::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if oid != target_oid {
            continue;
        }

        let mut vc = Cursor::new(&buf[val_off..val_off + KV_SIZE]);
        // flags(4) + size(4) + paddr(i64)
        let _flags = vc
            .read_u32::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let _size = vc
            .read_u32::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let paddr_raw = vc
            .read_i64::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if paddr_raw >= 0 && xid > best_xid {
            best_xid = xid;
            #[allow(clippy::cast_sign_loss)]
            // JUSTIFIED: We checked paddr_raw >= 0 immediately above.
            {
                best_paddr = Some(paddr_raw as u64);
            }
        }
    }

    best_paddr.ok_or_else(|| {
        AppError::Internal(format!("OMap: OID {target_oid} not found in leaf"))
    })
}

/// Traverse an OMap internal node to find the child containing `target_oid`.
///
/// In an OMap internal node the values are child node OIDs (physical), not
/// `(flags, size, paddr)` tuples. We do a linear scan to find the last key
/// whose `oid` <= `target_oid`, then follow that child pointer.
fn omap_internal_lookup(
    reader: &mut BlockReader,
    buf: &[u8],
    nkeys: u32,
    target_oid: u64,
) -> Result<u64, AppError> {
    const HDR_SIZE: usize = 72;
    const KEY_SIZE: usize = 16; // oid(8) + xid(8)
    const VAL_SIZE: usize = 8; // child paddr(u64) for internal nodes

    let nkeys = nkeys as usize;
    // Internal nodes have nkeys+1 child pointers (one per separator + rightmost)
    let keys_start = HDR_SIZE;
    let vals_start = HDR_SIZE + nkeys * KEY_SIZE;
    let required = vals_start + (nkeys + 1) * VAL_SIZE;

    if buf.len() < required {
        return Err(AppError::Internal(
            "OMap internal node buffer too small".to_string(),
        ));
    }

    // Find the largest key oid <= target_oid
    let mut child_index = 0usize;
    for i in 0..nkeys {
        let key_off = keys_start + i * KEY_SIZE;
        let mut kc = Cursor::new(&buf[key_off..key_off + KEY_SIZE]);
        let oid = kc
            .read_u64::<LE>()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if oid <= target_oid {
            child_index = i + 1;
        }
    }

    let val_off = vals_start + child_index * VAL_SIZE;
    let mut vc = Cursor::new(&buf[val_off..val_off + VAL_SIZE]);
    let child_block = vc
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    omap_btree_lookup(reader, child_block, target_oid)
}

// ---------------------------------------------------------------------------
// FS B-tree walk
// ---------------------------------------------------------------------------

/// Walk the FS B-tree rooted at `root_block`, collecting deleted inodes
/// (`nlink == 0`) and sending them through `tx`.
///
/// This is an MVP linear scan of leaf nodes only. It does not cross-reference
/// the space manager free bitmap — that is deferred to the probability engine.
///
/// # Errors
///
/// Returns `Err` if reading any node fails. Individual malformed entries are
/// skipped.
pub(crate) fn walk_fs_btree(
    reader: &mut BlockReader,
    root_block: u64,
    tx: &mpsc::Sender<DeletedFileEntry>,
) -> Result<(), AppError> {
    // MVP: depth-first traversal of the B-tree, collecting leaf nodes.
    walk_fs_node(reader, root_block, tx, 0)
}

/// Recursive B-tree traversal. `depth` guards against cycles (max 8 levels).
fn walk_fs_node(
    reader: &mut BlockReader,
    node_block: u64,
    tx: &mpsc::Sender<DeletedFileEntry>,
    depth: u8,
) -> Result<(), AppError> {
    if depth > 8 {
        return Ok(());
    }

    let buf = reader.read_block(node_block)?;
    let (level, nkeys) = read_btn_header(&buf)?;

    if nkeys == 0 {
        return Ok(());
    }

    if level == 0 {
        // Leaf: scan for INODE records with nlink == 0
        scan_fs_leaf_node(&buf, nkeys, tx);
    } else {
        // Internal: iterate child pointers
        walk_fs_internal_node(reader, &buf, nkeys, tx, depth);
    }

    Ok(())
}

/// Walk an FS B-tree internal node's children.
fn walk_fs_internal_node(
    reader: &mut BlockReader,
    buf: &[u8],
    nkeys: u32,
    tx: &mpsc::Sender<DeletedFileEntry>,
    depth: u8,
) {
    // FS B-tree internal node values are virtual OIDs (8 bytes each).
    // The key area uses variable-length keys in the FS B-tree, so we cannot
    // do a simple arithmetic layout here. Apple uses a table-of-contents
    // (TOC) in the 72-byte header to index variable key/value pairs.
    //
    // For the MVP we perform a linear scan of potential child block addresses
    // at evenly spaced offsets within the node, treating each u64 word in the
    // value area as a candidate block address. This is a best-effort approach
    // for deleted-file recovery where false negatives (missed files) are
    // acceptable but false positives are harmless (empty reads return no data).
    //
    // TODO(P3): implement full variable-key TOC traversal.

    // Value area in FS internal nodes starts at offset 72 + (nkeys * avg_key_size).
    // Since keys are variable, we use a heuristic: scan the latter half of the
    // node for valid-looking block addresses and recurse.
    let node_size = buf.len();
    let search_start = node_size / 2;
    let step = 8usize;

    let mut i = search_start;
    while i + 8 <= node_size {
        let mut vc = Cursor::new(&buf[i..i + 8]);
        if let Ok(candidate) = vc.read_u64::<LE>() {
            // Heuristic: block addresses in a reasonable range
            let max_block = 0x0010_0000_0000u64; // 4 TB at 4096-byte blocks
            if candidate > 0 && candidate < max_block {
                // Ignore errors from individual children — keep walking
                let _ = walk_fs_node(reader, candidate, tx, depth + 1);
            }
        }
        i += step;
    }

    let _ = nkeys; // nkeys used for direction — actual traversal above is heuristic
}

/// Scan an FS B-tree leaf node for INODE records with nlink == 0.
///
/// The FS B-tree uses variable-length keys and values. The minimum key is
/// 8 bytes (obj_id_and_type: u64). We scan from offset 72 forward, looking
/// for APFS_TYPE_INODE keys. This is an MVP heuristic scanner.
fn scan_fs_leaf_node(
    buf: &[u8],
    nkeys: u32,
    tx: &mpsc::Sender<DeletedFileEntry>,
) {
    if buf.len() < 80 {
        return;
    }

    // Walk the buffer 8 bytes at a time looking for plausible INODE keys.
    // A valid INODE key has bits 60-63 == APFS_TYPE_INODE (0x3).
    let inode_type_mask: u64 = u64::from(APFS_TYPE_INODE) << 60;
    let type_mask: u64 = 0xF << 60;

    let mut offset = 72usize;
    while offset + 8 <= buf.len() {
        let mut kc = Cursor::new(&buf[offset..offset + 8]);
        let Ok(obj_id_and_type) = kc.read_u64::<LE>() else { break };

        if (obj_id_and_type & type_mask) == inode_type_mask {
            let inode_id = obj_id_and_type & !(type_mask);
            // Try to read the inode value immediately following the 8-byte key
            if let Some(entry) = try_parse_inode_value(buf, offset + 8, inode_id) {
                if entry.nlink == 0 {
                    let deleted = inode_to_deleted_entry(entry);
                    // Non-blocking try_send — if receiver is gone, stop sending
                    if tx.try_send(deleted).is_err() {
                        return;
                    }
                }
            }
        }

        offset += 8;
    }

    let _ = nkeys;
}

/// Attempt to parse an inode value record starting at `offset` within `buf`.
///
/// Returns `None` if there are insufficient bytes or the data looks invalid.
///
/// Inode value (`j_inode_val_t`) layout (subset we care about):
///   parent_id(8) + private_id(8) + create_time(8) + mod_time(8)
///   + change_time(8) + access_time(8) + internal_flags(8)
///   + nchildren_or_nlink(4) + default_protection_class(4)
///   + write_generation_counter(4) + bsd_flags(4)
///   + owner(4) + group(4) + mode(2) + pad1(2) + uncompressed_size(8)
fn try_parse_inode_value(buf: &[u8], offset: usize, inode_id: u64) -> Option<InodeRecord> {
    // Minimum inode value size we need to parse nlink
    const MIN_INODE_VAL: usize = 8 + 8 + 8 + 8 + 8 + 8 + 8 + 4; // 60 bytes

    if offset + MIN_INODE_VAL > buf.len() {
        return None;
    }

    let mut cur = Cursor::new(&buf[offset..]);

    let _parent_id = cur.read_u64::<LE>().ok()?;
    let _private_id = cur.read_u64::<LE>().ok()?;
    let _create_time = cur.read_u64::<LE>().ok()?;
    let mod_time_raw = cur.read_u64::<LE>().ok()?;
    let _change_time = cur.read_u64::<LE>().ok()?;
    let _access_time = cur.read_u64::<LE>().ok()?;
    let _internal_flags = cur.read_u64::<LE>().ok()?;
    let nlink_raw = cur.read_u32::<LE>().ok()?;

    // Sanity check: nlink > 1000 is almost certainly a parse artefact
    let nlink = if nlink_raw > 1000 { return None; } else { nlink_raw };

    // mod_time is nanoseconds since UNIX epoch in APFS
    let mod_time_nanos = if mod_time_raw == 0 {
        None
    } else {
        Some(mod_time_raw)
    };

    Some(InodeRecord {
        inode_id,
        name: None,
        size: 0,
        mod_time_nanos,
        nlink,
        extents: vec![],
    })
}

/// Convert a parsed `InodeRecord` to a `DeletedFileEntry`.
fn inode_to_deleted_entry(rec: InodeRecord) -> DeletedFileEntry {
    use std::time::{Duration, UNIX_EPOCH};

    let deleted_at = rec.mod_time_nanos.map(|ns| {
        UNIX_EPOCH + Duration::from_nanos(ns)
    });

    DeletedFileEntry {
        inode_id: rec.inode_id,
        name: rec.name,
        size_bytes: rec.size,
        deleted_at,
        extents: rec.extents,
        filesystem: FileSystem::APFS,
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Advance a `Cursor` by `n` bytes, returning `Err` if it would overflow.
fn advance(cur: &mut Cursor<&[u8]>, n: u64) -> Result<(), AppError> {
    let new_pos = cur
        .position()
        .checked_add(n)
        .ok_or_else(|| AppError::Internal("Cursor position overflow".to_string()))?;
    cur.set_position(new_pos);
    Ok(())
}

/// Parse a null-terminated byte slice as a UTF-8 string, replacing invalid
/// sequences with the Unicode replacement character.
fn parse_cstring(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a 1024-byte buffer with a valid NX superblock at the correct offsets.
    ///
    /// Layout offsets (little-endian):
    ///   0–31:   obj_phys_t header (zeroed)
    ///   32–35:  nx_magic = NX_MAGIC
    ///   36–39:  nx_block_size = 4096
    ///   40–47:  nx_block_count = 1000
    ///   48–103: features/uuid/ids (zeroed) — 56 bytes
    ///   104–107: nx_xp_desc_blocks = 8
    ///   108–111: nx_xp_data_blocks = 0
    ///   112–119: nx_xp_desc_base = 2 (i64 LE)
    ///   120–127: nx_xp_data_base (zeroed)
    ///   128–131: nx_xp_desc_next
    ///   132–135: nx_xp_data_next
    ///   136–139: nx_xp_desc_index
    ///   140–143: nx_xp_desc_len = 4
    ///   144–147: nx_xp_data_len
    ///   148–155: nx_spaceman_oid (zeroed)
    ///   156–163: nx_omap_oid = 5
    ///   164–171: nx_reaper_oid (zeroed)
    ///   172–175: nx_test_type (zeroed)
    ///   176–179: nx_max_file_systems = 1
    ///   180–187: nx_fs_oid[0] = 7
    fn make_valid_nx_buf() -> Vec<u8> {
        let mut buf = vec![0u8; 1024];
        // nx_magic at offset 32
        buf[32..36].copy_from_slice(&NX_MAGIC.to_le_bytes());
        // nx_block_size at offset 36
        buf[36..40].copy_from_slice(&4096u32.to_le_bytes());
        // nx_block_count at offset 40
        buf[40..48].copy_from_slice(&1000u64.to_le_bytes());
        // [48..104]: zeroed (56 bytes skipped)
        // nx_xp_desc_blocks at 104
        buf[104..108].copy_from_slice(&8u32.to_le_bytes());
        // nx_xp_data_blocks at 108 (zeroed)
        // nx_xp_desc_base at 112
        buf[112..120].copy_from_slice(&2i64.to_le_bytes());
        // nx_xp_data_base 120-127 zeroed
        // nx_xp_desc_next 128-131, nx_xp_data_next 132-135, nx_xp_desc_index 136-139 zeroed
        // nx_xp_desc_len at 140
        buf[140..144].copy_from_slice(&4u32.to_le_bytes());
        // nx_xp_data_len 144-147 zeroed
        // nx_spaceman_oid 148-155 zeroed
        // nx_omap_oid at 156
        buf[156..164].copy_from_slice(&5u64.to_le_bytes());
        // nx_reaper_oid 164-171 zeroed
        // nx_test_type 172-175 zeroed
        // nx_max_file_systems at 176
        buf[176..180].copy_from_slice(&1u32.to_le_bytes());
        // nx_fs_oid[0] at 180
        buf[180..188].copy_from_slice(&7u64.to_le_bytes());
        buf
    }

    #[derive(Debug)]
    struct NxParseCase {
        name: &'static str,
        buf: Vec<u8>,
        expect_ok: bool,
    }

    #[test]
    fn test_parse_nx_superblock() {
        let mut wrong_magic = make_valid_nx_buf();
        wrong_magic[32..36].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());

        let mut bad_block_size = make_valid_nx_buf();
        bad_block_size[36..40].copy_from_slice(&0u32.to_le_bytes());

        let mut bad_alignment = make_valid_nx_buf();
        bad_alignment[36..40].copy_from_slice(&1000u32.to_le_bytes());

        let cases = vec![
            NxParseCase {
                name: "parse_nx_valid_magic",
                buf: make_valid_nx_buf(),
                expect_ok: true,
            },
            NxParseCase {
                name: "parse_nx_wrong_magic",
                buf: wrong_magic,
                expect_ok: false,
            },
            NxParseCase {
                name: "parse_nx_too_small",
                buf: vec![0u8; 100],
                expect_ok: false,
            },
            NxParseCase {
                name: "parse_nx_invalid_block_size",
                buf: bad_block_size,
                expect_ok: false,
            },
            NxParseCase {
                name: "parse_nx_block_size_not_aligned",
                buf: bad_alignment,
                expect_ok: false,
            },
        ];

        for case in cases {
            let result = parse_nx_superblock(&case.buf);
            assert_eq!(
                result.is_ok(),
                case.expect_ok,
                "FAILED case: {} — got {:?}",
                case.name,
                result
            );
        }
    }

    #[test]
    fn test_parse_nx_superblock_fields() {
        let sb = parse_nx_superblock(&make_valid_nx_buf())
            // JUSTIFIED: test-only; fixture is a valid superblock
            .expect("valid fixture must parse");
        assert_eq!(sb.block_size, 4096, "block_size mismatch");
        assert_eq!(sb.block_count, 1000, "block_count mismatch");
        assert_eq!(sb.omap_oid, 5, "omap_oid mismatch");
        assert_eq!(sb.fs_oids, vec![7u64], "fs_oids mismatch");
        assert_eq!(sb.xp_desc_base, 2, "xp_desc_base mismatch");
        // xp_desc_len = min(4, 8) = 4
        assert_eq!(sb.xp_desc_len, 4, "xp_desc_len mismatch");
    }

    // ---------------------------------------------------------------------------
    // disk_id_to_raw_device tests live in mod.rs — tested there
    // ---------------------------------------------------------------------------

    #[test]
    fn test_parse_cstring_null_terminated() {
        #[derive(Debug)]
        struct Case {
            name: &'static str,
            input: Vec<u8>,
            expected: &'static str,
        }

        let cases = vec![
            Case {
                name: "happy_path_normal_string",
                input: b"hello\0world".to_vec(),
                expected: "hello",
            },
            Case {
                name: "branch_no_null_terminator",
                input: b"abc".to_vec(),
                expected: "abc",
            },
            Case {
                name: "branch_empty",
                input: vec![0u8],
                expected: "",
            },
        ];

        for case in cases {
            let actual = parse_cstring(&case.input);
            assert_eq!(actual, case.expected, "FAILED case: {}", case.name);
        }
    }

    #[test]
    fn test_advance_overflow() {
        let data = vec![0u8; 8];
        let mut cur = Cursor::new(data.as_slice());
        cur.set_position(u64::MAX);
        let result = advance(&mut cur, 1);
        assert!(result.is_err(), "expected overflow error");
    }

    #[test]
    fn test_read_btn_header_too_small() {
        let buf = vec![0u8; 20]; // less than 40 bytes
        let result = read_btn_header(&buf);
        assert!(result.is_err(), "expected error on small buffer");
    }

    #[test]
    fn test_omap_leaf_scan_empty() {
        // A leaf node with nkeys=0 should return Err (OID not found)
        let buf = vec![0u8; 256];
        let result = omap_leaf_scan(&buf, 0, 42);
        assert!(result.is_err(), "expected not-found error for empty leaf");
    }
}
