# 10-p2-hfsplus-parser
**Task ID:** P2-T02  
**Phase:** P2 — Filesystem Parsers  
**Owner:** 🔵 Developer  
**Status:** In Progress

---

## Overview

Parse HFS+ (Mac OS Extended) volumes from a raw block device (`/dev/rdiskN`) and stream deleted file metadata via `tokio::sync::mpsc::Sender<DeletedFileEntry>`. HFS+ uses a B*-tree Catalog File as its primary metadata structure; deleted file candidates are found by identifying file records whose data extents overlap free allocation bitmap blocks.

## Scope

- `crates/disk/src/macos/hfsplus/` module (mod.rs, structs.rs, scanner.rs)
- Volume Header parsing (magic check, block size, catalog + alloc extents)
- Allocation bitmap read (free-block detection)
- Catalog B*-tree leaf-node traversal (file records only, MVP)
- `CatalogDeletedFile` → `DeletedFileEntry` conversion
- Public async `scan_hfsplus` + sync `scan_hfsplus_sync` entry points
- `byteorder = "1"` added to `[target.'cfg(target_os = "macos")'.dependencies]`
- Table-driven unit tests for all parsers (no disk access required)

## Out of Scope

- Index node traversal (MVP walks linear extent; full B-tree traversal stubbed)
- HFSX journal replay
- Resource fork recovery
- Windows platform

## Dependencies

- Blocked by: P2-T01 (APFS parser) — crate structure already in place; HFS+ adds a sibling module

---

## Developer Plan

### Module structure

```
crates/disk/src/macos/hfsplus/
├── mod.rs       — public scan entry point (scan_hfsplus, scan_hfsplus_sync)
├── structs.rs   — constants, HfsPlusVolumeHeader, CatalogDeletedFile
└── scanner.rs   — parse_volume_header, read_alloc_bitmap, walk_catalog_leaves, is_block_allocated
```

### Key function signatures

```rust
// structs.rs
pub fn parse_volume_header(buf: &[u8]) -> Result<HfsPlusVolumeHeader, AppError>;
pub fn is_block_allocated(alloc_bitmap: &[u8], block_num: u32) -> bool;

// scanner.rs
pub(crate) fn read_extents(device: &mut std::fs::File, extents: &[(u32, u32)], block_size: u32) -> Result<Vec<u8>, AppError>;
pub(crate) fn walk_catalog_leaves(catalog_data: &[u8], node_size: u32, block_size: u32, alloc_bitmap: &[u8]) -> Result<Vec<CatalogDeletedFile>, AppError>;

// mod.rs
pub fn scan_hfsplus_sync(disk_id: &str, tx: mpsc::Sender<DeletedFileEntry>) -> Result<(), AppError>;
pub async fn scan_hfsplus(disk_id: String, tx: mpsc::Sender<DeletedFileEntry>) -> Result<(), AppError>;
```

### HFS+ Volume Header offsets (all big-endian)

| Offset | Size | Field |
|--------|------|-------|
| 0 | 2 | signature |
| 2 | 2 | version |
| 40 | 4 | blockSize |
| 44 | 4 | totalBlocks |
| 48 | 4 | freeBlocks |
| 112 | 8 | allocationFile.logicalSize |
| 120 | 4 | allocationFile.clumpSize |
| 124 | 4 | allocationFile.totalBlocks |
| 128 | 80 | allocationFile.extents (8 × 8 bytes) |
| 208 | 8 | catalogFile.logicalSize |
| 216 | 4 | catalogFile.clumpSize |
| 220 | 4 | catalogFile.totalBlocks |
| 224 | 80 | catalogFile.extents (8 × 8 bytes) |

## Edge Cases

- Magic mismatch → `Err(AppError::Internal)` — never panic on bad disk data
- Buffer too small → `Err` before any field reads
- Zero block_size → guard against divide-by-zero in extent calculations
- Allocation bitmap shorter than claimed block count → treat out-of-range as free
- Catalog node with zero records → skip cleanly
- HFS+ time before Unix epoch (1904–1970) → `None` for `deleted_at`
- `disk_id` already has `r` prefix or wrong format → handle path construction defensively

## Test Plan

| Case | Input | Expected | Branch |
|------|-------|----------|--------|
| `parse_volume_header_valid` | 512-byte buf, magic=0x482B, blockSize=4096 @40 | `Ok(block_size=4096)` | happy path |
| `parse_volume_header_hfsx_magic` | magic=0x4858 | `Ok(...)` | HFSX variant |
| `parse_volume_header_wrong_magic` | magic=0xFFFF | `Err(...)` | bad magic |
| `parse_volume_header_too_small` | 100-byte buf | `Err(...)` | short buffer |
| `is_block_allocated_set` | bitmap[0]=0x80, block=0 | `true` | MSB set |
| `is_block_allocated_clear` | bitmap[0]=0x80, block=1 | `false` | MSB unset |
| `is_block_allocated_out_of_range` | empty bitmap, block=0 | `false` | out of range |
| `hfs_time_conversion_valid` | hfs_time=0x7C25B080 | `Some(SystemTime > UNIX_EPOCH)` | valid time |
| `hfs_time_conversion_pre_epoch` | hfs_time=0 (1904-01-01) | `None` | underflow |

---

## Implementation Notes

[To be filled after implementation]

## Open Questions / TPM Queries

None currently.

---

## QA Sign-off

[Pending]

## Security Sign-off

[N/A — no unsafe blocks in this task]
