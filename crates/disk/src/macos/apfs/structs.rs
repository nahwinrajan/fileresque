/// APFS container superblock magic: "NXSB" in little-endian bytes.
pub const NX_MAGIC: u32 = 0x4253_584e;

/// APFS volume superblock magic: "APSB" in little-endian bytes.
pub const APFS_MAGIC: u32 = 0x4253_5041;

/// Minimum valid block size (512 bytes — smallest supported sector size).
pub const MIN_BLOCK_SIZE: u32 = 512;

/// Maximum valid block size (64 KiB — Apple never ships larger).
pub const MAX_BLOCK_SIZE: u32 = 65536;

/// APFS J-key type for inode records (bits 60–63 of `obj_id_and_type`).
pub const APFS_TYPE_INODE: u8 = 3;

/// APFS J-key type for directory record entries (bits 60–63).
pub const APFS_TYPE_DIR_REC: u8 = 9;

/// Byte offset of the `nx_magic` field within an `nx_superblock_t`.
///
/// Layout before magic:
///   obj_phys_t header = 32 bytes (checksum 8 + oid 8 + xid 8 + type 4 + subtype 4)
pub const NX_SUPERBLOCK_MAGIC_OFFSET: u64 = 32;

/// Parsed container superblock — only the fields required for APFS scanning.
#[derive(Debug, Clone)]
pub struct NxSuperblock {
    /// Block size in bytes for all blocks in this container.
    pub block_size: u32,
    /// Total number of blocks in the container.
    pub block_count: u64,
    /// Object ID of the container object map.
    pub omap_oid: u64,
    /// Ephemeral object ID of the container space manager (`nx_spaceman_oid`).
    /// Resolved to a physical address via the checkpoint descriptor area.
    pub spaceman_oid: u64,
    /// Object IDs of APFS volumes present in this container (non-zero entries).
    pub fs_oids: Vec<u64>,
    /// Physical block address of the checkpoint descriptor area.
    pub xp_desc_base: u64,
    /// Number of blocks in the checkpoint descriptor area (clamped to actual).
    pub xp_desc_len: u32,
}

/// Parsed APFS volume superblock — fields required for B-tree traversal.
#[derive(Debug, Clone)]
pub struct ApfsSuperblock {
    /// Human-readable volume name (UTF-8, may be empty).
    pub volume_name: String,
    /// Object ID of this volume's object map.
    pub omap_oid: u64,
    /// Object ID of this volume's root file-system B-tree.
    pub root_tree_oid: u64,
}

/// A parsed inode record extracted from an APFS file-system B-tree leaf node.
#[derive(Debug)]
pub struct InodeRecord {
    /// The inode number.
    pub inode_id: u64,
    /// File name, if a directory record was found for this inode.
    pub name: Option<String>,
    /// Logical file size in bytes.
    pub size: u64,
    /// Last modification time in nanoseconds since UNIX epoch.
    pub mod_time_nanos: Option<u64>,
    /// Hard link count — candidates with `nlink == 0` are deleted.
    pub nlink: u32,
    /// Data block extents: each entry is `(block_addr, block_count)`.
    pub extents: Vec<(u64, u64)>,
}
