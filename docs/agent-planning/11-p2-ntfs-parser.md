# Planning Doc — P2-T03 NTFS Parser (Windows)

**Task ID:** P2-T03  
**Phase:** P2 — Filesystem Parsers  
**Status:** In Progress  
**Agent:** 🔵 Developer  
**Security Gate:** Yes — all `unsafe` blocks require `// SAFETY:` (none expected; this task avoids unsafe)

---

## Background

FileResque needs to scan NTFS volumes on Windows and return a stream of deleted-file metadata. NTFS stores all file metadata in the Master File Table (MFT). Deleted files are MFT records where the `IN_USE` flag (bit 0 of the `flags` field) is 0.

This code targets Windows (`#[cfg(target_os = "windows")]`) but is developed on macOS. All unit tests exercise pure parsing logic with synthetic byte buffers — no Win32 API calls in tests. Tests run on the macOS CI host via `#[cfg(any(target_os = "windows", test))]`.

---

## Developer Plan

### Module structure

```
crates/disk/src/windows/ntfs/
├── mod.rs      — public async scan_ntfs entry point
├── structs.rs  — NTFS constants and parsed types
└── scanner.rs  — VBR parsing, MFT record parsing, attribute parsing, scan loop
```

`windows/mod.rs` adds `pub mod ntfs;`.

### Dependencies added

- `byteorder = "1"` under `[dependencies]` in `crates/disk/Cargo.toml` (not platform-gated — tests need it on macOS)
- Workspace `[workspace.lints.rust]` declares `fuzzing` as a known cfg to suppress `unexpected_cfgs` warnings when using `cfg(fuzzing)` for the fuzz target gate

### Key design decisions

| Decision | Rationale |
|----------|-----------|
| `#[cfg(any(target_os = "windows", test, fuzzing))]` on all parsing items | Parsing is pure bytes; gating avoids dead-code warnings in macOS production builds while enabling tests on any platform and fuzzing |
| `#[cfg(target_os = "windows")]` on `scan_mft` | I/O logic opens Windows device paths; cannot run on macOS |
| `extents: vec![]` in `DeletedFileEntry` | NTFS data-run parsing (LCN mapping) is complex; deferred to future task. Extent field is populated as empty for MVP |
| `filetime_to_system_time` returning `Option` | Windows FILETIME values before Unix epoch yield `None` without panic |
| `find_best_file_name` prefers Win32/Win32AndDos namespace | Files stored under POSIX or DOS namespace only have limited metadata; Win32 names are the user-visible names |

### Function signatures

```rust
// structs.rs — all under cfg(any(target_os = "windows", test, fuzzing))
pub struct NtfsVbr { ... }
pub struct MftRecord { ... }
pub struct AttrHeader { ... }
pub struct FileNameAttr { ... }
pub struct NtfsDeletedFile { ... }

// scanner.rs — all under cfg(any(target_os = "windows", test, fuzzing))
pub fn parse_vbr(buf: &[u8]) -> Result<NtfsVbr, AppError>
pub fn mft_record_size(vbr: &NtfsVbr) -> usize
pub fn parse_mft_record(buf: &[u8], record_number: u64) -> Result<MftRecord, AppError>
pub fn is_deleted(record: &MftRecord) -> bool
pub fn parse_attr_header(buf: &[u8], offset: usize) -> Result<Option<AttrHeader>, AppError>
pub fn parse_file_name_attr(data: &[u8]) -> Result<FileNameAttr, AppError>
pub fn filetime_to_system_time(filetime: u64) -> Option<std::time::SystemTime>
pub(crate) fn find_best_file_name(buf: &[u8], record: &MftRecord) -> Option<FileNameAttr>
pub(crate) fn extract_deleted_entry(buf: &[u8], record: &MftRecord, vbr: &NtfsVbr) -> Option<DeletedFileEntry>

// scan_mft — cfg(target_os = "windows") only
pub(crate) fn scan_mft(device_path: &str, tx: &mpsc::Sender<DeletedFileEntry>) -> Result<(), AppError>

// ntfs/mod.rs — public async API
pub async fn scan_ntfs(device_path: String, tx: mpsc::Sender<DeletedFileEntry>) -> Result<(), AppError>
```

### Edge cases

| Case | Handling |
|------|----------|
| VBR buffer < 512 bytes | `Err(AppError::Internal(...))` |
| VBR OEM ID != "NTFS    " | `Err(AppError::Internal(...))` |
| MFT record magic mismatch | `Err(AppError::Internal(...))` |
| MFT record buffer < 48 bytes | `Err(AppError::Internal(...))` |
| `clusters_per_mft_record < 0` | Record size = `2^|value|` bytes |
| Attribute length == 0 or overflow | `Err(AppError::Internal(...))` |
| Attribute value beyond buffer | `Err(AppError::Internal(...))` |
| `$FILE_NAME` buffer < 66 bytes | `Err(AppError::Internal(...))` |
| `$FILE_NAME` name bytes out of bounds | `Err(AppError::Internal(...))` |
| Invalid UTF-16 in file name | `Err(AppError::Internal(...))` |
| FILETIME before Unix epoch | `filetime_to_system_time` returns `None` |
| MFT record with bad magic during scan | Skip record, increment counter, continue |
| `tx.blocking_send` fails (scan cancelled) | Silently ignored (`let _ = ...`) |
| Device path permission denied | `AppError::PermissionDenied` |

---

## Task Completion Checklist

- [ ] `cargo clippy` — 0 warnings
- [ ] `cargo fmt` — clean
- [ ] Cognitive complexity ≤ 15 on all new functions
- [ ] Unit tests written (table-driven); coverage ≥ 80%
- [ ] No `unwrap()`/`expect()` without `// JUSTIFIED:`
- [ ] All `unsafe` blocks have `// SAFETY:` (none expected)
- [ ] Planning doc updated with `## Implementation Notes`
- [ ] 🟢 QA sign-off appended to planning doc
- [ ] 🔴 Security sign-off appended (security gate applies)
