# P2-T01 — APFS Parser (macOS)

**Phase:** 2  
**Status:** In Progress  
**Security Gate:** Yes — SEC review required for any unsafe blocks  
**Agent:** Developer (🔵)

---

## Task Scope

Parse APFS volumes from a raw block device (`/dev/rdiskN`) and emit deleted file
metadata via `tokio::sync::mpsc`. All raw disk I/O is synchronous and wrapped
in `tokio::task::spawn_blocking`.

---

## Approach

- Use `byteorder::LE` reads via `std::io::Cursor` — no `#[repr(C)]` transmutes,
  no `unsafe` required for integer parsing.
- MVP scan: container superblock → OMap (object map) → volume superblock →
  FS B-tree leaf nodes → collect inodes with `nlink == 0`.
- Space-manager cross-reference is **stubbed** for this MVP; probability engine
  (Phase 3) handles that.
- OMap B-tree traversal is implemented as a linear leaf scan rather than a full
  recursive B-tree descent.

---

## Module Structure

```
crates/disk/src/macos/apfs/
├── mod.rs        — public API: scan_apfs / scan_apfs_sync / disk_id_to_raw_device
├── structs.rs    — APFS binary struct definitions and constants
├── reader.rs     — BlockReader (open device, read_block)
└── scanner.rs    — parse_nx_superblock, parse_apfs_superblock, omap_lookup,
                    walk_fs_btree
```

---

## Function Signatures

```rust
// mod.rs
pub async fn scan_apfs(disk_id: String, tx: mpsc::Sender<DeletedFileEntry>) -> Result<(), AppError>
pub fn scan_apfs_sync(disk_id: &str, tx: mpsc::Sender<DeletedFileEntry>) -> Result<(), AppError>
pub(crate) fn disk_id_to_raw_device(disk_id: &str) -> Result<String, AppError>

// reader.rs
pub struct BlockReader { pub block_size: u32 }
impl BlockReader {
    pub fn open(device_path: &str) -> Result<Self, AppError>
    pub fn read_block(&mut self, block_addr: u64) -> Result<Vec<u8>, AppError>
}

// scanner.rs
pub fn parse_nx_superblock(buf: &[u8]) -> Result<NxSuperblock, AppError>
pub(crate) fn parse_apfs_superblock(buf: &[u8]) -> Result<ApfsSuperblock, AppError>
pub(crate) fn omap_lookup(reader: &mut BlockReader, omap_root_block: u64, target_oid: u64) -> Result<u64, AppError>
pub(crate) fn walk_fs_btree(reader: &mut BlockReader, root_block: u64, tx: &mpsc::Sender<DeletedFileEntry>) -> Result<(), AppError>
```

---

## Edge Cases

- Buffer too small for superblock → `Err(AppError::Internal(...))`
- Magic mismatch → `Err` with diagnostic hex values
- Block address overflow on multiplication → checked arithmetic
- Non-numeric disk suffix (e.g. `"diskX"`) → `Err`
- Invalid block size (0, non-multiple of 512, >65536) → `Err`
- Negative `xp_desc_base` → `Err` (sign-extended pointer from Apple)
- Empty `fs_oids` → return `Ok(())` with no entries emitted
- OMap lookup failure → skip volume, continue
- B-tree node with zero keys → skip node

---

## Test Matrix

| Test | Expected |
|------|----------|
| `parse_nx_valid_magic` | `Ok(NxSuperblock { block_size: 4096, ... })` |
| `parse_nx_wrong_magic` | `Err(AppError::Internal(...))` |
| `parse_nx_too_small` | `Err(...)` |
| `parse_nx_invalid_block_size` | `Err(...)` |
| `parse_nx_block_size_not_aligned` | `Err(...)` |
| `disk_id_to_raw_device_valid` | `Ok("/dev/rdisk0")` |
| `disk_id_to_raw_device_multi_digit` | `Ok("/dev/rdisk12")` |
| `disk_id_to_raw_device_invalid` | `Err(...)` |
| `disk_id_to_raw_device_non_numeric` | `Err(...)` |

---

## Security Notes

- No `unsafe` blocks — `byteorder` handles all LE integer reads safely.
- All filename/path data from disk is treated as untrusted bytes.
- Block address arithmetic uses `checked_mul` to prevent overflow.
- Buffer bounds checked before every read via `Cursor` returning `Err` on EOF.

---

## Implementation Notes

_To be filled after implementation._

---

## Completion Checklist

- [ ] `cargo clippy` — 0 warnings
- [ ] `cargo fmt` — clean
- [ ] Cognitive complexity ≤ 15 on all new functions
- [ ] Unit tests written (table-driven); coverage ≥ 80%
- [ ] No `unwrap()`/`expect()` without `// JUSTIFIED:`
- [ ] All `unsafe` blocks have `// SAFETY:`
- [ ] Planning doc updated with `## Implementation Notes`
- [ ] 🟢 QA sign-off appended to planning doc
- [ ] 🔴 Security sign-off appended (security gate applies)
