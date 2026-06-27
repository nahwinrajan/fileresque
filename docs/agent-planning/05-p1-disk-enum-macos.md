# 05-p1-disk-enum-macos
**Task ID:** P1-T01
**Phase:** P1 — Disk Discovery
**Owner:** [DEV]
**Status:** QA Review

---

## Overview

Implement macOS disk enumeration using `diskutil list -plist physical` and `diskutil info -plist /dev/diskN`. The result is a `Vec<DiskInfo>` surfaced to the frontend via the `get_disks` Tauri command. This replaces the P0 `greet` placeholder command.

## Scope

- `list_disks` async entry point in `crates/disk/src/macos/enumerate.rs`
- Synchronous implementation `list_disks_sync` (called via `spawn_blocking`)
- `classify_stderr_error` — testable error classifier for `diskutil` stderr
- `parse_disk_info_from_dict` — testable plist dictionary → `DiskInfo` parser
- `detect_filesystem` — maps plist keys to `FileSystem` enum
- `detect_drive_type` — maps plist keys to `DriveType` enum
- `get_disks` Tauri command in `src-tauri/src/commands/mod.rs`
- Wire `get_disks` into `invoke_handler` in `src-tauri/src/lib.rs`
- Remove `greet` placeholder command
- Add `plist = "1"` macOS-target dependency to `crates/disk/Cargo.toml`
- 16 table-driven unit tests covering every branch

## Out of Scope

- IOKit bindings (not needed; `diskutil -plist` provides all required metadata)
- Windows disk enumeration (P1-T02)
- Frontend disk-list UI (P1-T03)
- Permission onboarding UI (P1-T04)

## Dependencies

- Blocked by: P0-T01 (project scaffold) — DONE

---

## Developer Plan

### Module Structure

```
crates/disk/src/macos/enumerate.rs
├── pub async fn list_disks()                        — async wrapper
├── pub(crate) fn list_disks_sync()                  — synchronous impl
├── fn run_diskutil(args: &[&str])                   — spawns diskutil process
├── pub(crate) fn classify_stderr_error(stderr, code) — error classifier
├── fn parse_plist(data: &[u8])                      — bytes → plist::Value
├── fn extract_physical_disk_ids(plist_value)        — extracts WholeDisks
├── fn disk_info_for(disk_id)                        — per-disk orchestrator
├── pub(crate) fn parse_disk_info_from_dict(id, dict) — dict → DiskInfo
├── fn detect_filesystem(dict)                       — FilesystemType/Name → FileSystem
└── fn detect_drive_type(dict)                       — SolidState/BusProtocol → DriveType
```

### Function Signatures

```rust
pub async fn list_disks() -> Result<Vec<DiskInfo>, AppError>
pub(crate) fn list_disks_sync() -> Result<Vec<DiskInfo>, AppError>
fn run_diskutil(args: &[&str]) -> Result<Vec<u8>, AppError>
pub(crate) fn classify_stderr_error(stderr: &str, exit_code: i32) -> AppError
fn parse_plist(data: &[u8]) -> Result<plist::Value, AppError>
fn extract_physical_disk_ids(plist_value: &plist::Value) -> Result<Vec<String>, AppError>
fn disk_info_for(disk_id: &str) -> Result<DiskInfo, AppError>
pub(crate) fn parse_disk_info_from_dict(disk_id: &str, dict: &plist::Dictionary) -> Result<DiskInfo, AppError>
fn detect_filesystem(dict: &plist::Dictionary) -> FileSystem
fn detect_drive_type(dict: &plist::Dictionary) -> DriveType
```

### `diskutil` Key Mapping

| DiskInfo field | diskutil plist key | Fallback |
|---|---|---|
| `display_name` | `MediaName` | `disk_id` |
| `size_bytes` | `TotalSize` (u64) | 0 |
| `encrypted` | `Encryption` (bool) | false |
| `trim_enabled` | `TrimSupport` (bool) | false |
| `mount_points` | `MountPoint` (str, may be empty) | `[]` |
| `serial` | `IORegistryEntryName` | None |
| `filesystem` | `FilesystemType` + `FilesystemName` | Unknown |
| `drive_type` | `BusProtocol` + `SolidState` | Unknown |

### FileSystem Detection Logic

Combined lowercase string from `FilesystemType + " " + FilesystemName`:
- `"apfs"` → `APFS`
- `"hfs"` or `"mac os extended"` → `HFSPlus`
- `"fat32"` or `"msdos"` → `FAT32`
- `"exfat"` → `ExFAT`
- `"ntfs"` → `NTFS`
- none match → `Unknown`

### DriveType Detection Logic

`BusProtocol` (lowercased) + `SolidState` boolean:
- protocol contains `"usb"` → `USB`
- protocol contains `"nvme"` or `"pcie"` → `NVMe`
- `SolidState == true` (no USB/NVMe) → `SSD`
- protocol contains `"sata"` or `"ata"` → `HDD`
- none match → `Unknown`

## Edge Cases

- Disk ejected between `list` and `info` calls: non-`PermissionDenied` errors on individual disks are skipped via `continue`
- Empty `WholeDisks` array: returns `Ok(vec![])`
- `MountPoint` empty string: filtered out, `mount_points` remains empty `Vec`
- `TotalSize` missing: defaults to 0
- `Encryption` / `TrimSupport` missing: defaults to `false`
- Permission denied on any disk: immediately propagates as `AppError::PermissionDenied`
- Non-UTF-8 in plist: `from_utf8_lossy` used for stderr; `plist::from_bytes` handles plist binary correctly

## Test Plan

| Case | Input | Expected | Branch covered |
|------|-------|----------|----------------|
| `parse_whole_disks_empty` | plist with `WholeDisks: []` | `Ok(vec![])` | empty array |
| `parse_whole_disks_single` | plist with `WholeDisks: ["disk0"]` | `Ok(vec!["disk0"])` | happy path |
| `detect_filesystem_apfs` | dict with `FilesystemType: "apfs"` | `FileSystem::APFS` | APFS branch |
| `detect_filesystem_hfsplus` | dict with `FilesystemName: "Mac OS Extended"` | `FileSystem::HFSPlus` | HFSPlus branch |
| `detect_filesystem_fat32` | dict with `FilesystemType: "msdos"` | `FileSystem::FAT32` | FAT32 branch |
| `detect_filesystem_exfat` | dict with `FilesystemType: "exfat"` | `FileSystem::ExFAT` | ExFAT branch |
| `detect_filesystem_unknown` | empty dict | `FileSystem::Unknown` | fallback |
| `detect_drive_type_usb` | dict with `BusProtocol: "USB"` | `DriveType::USB` | USB branch |
| `detect_drive_type_nvme` | dict with `BusProtocol: "NVMe"` | `DriveType::NVMe` | NVMe branch |
| `detect_drive_type_ssd` | dict with `SolidState: true`, no protocol | `DriveType::SSD` | SSD branch |
| `detect_drive_type_hdd` | dict with `SolidState: false`, `BusProtocol: "SATA"` | `DriveType::HDD` | HDD branch |
| `detect_drive_type_unknown` | empty dict | `DriveType::Unknown` | fallback |
| `disk_info_encrypted` | dict with `Encryption: true` | `DiskInfo { encrypted: true }` | encryption field |
| `disk_info_trim` | dict with `TrimSupport: true` | `DiskInfo { trim_enabled: true }` | trim field |
| `run_diskutil_permission_error` | stderr `"Operation not permitted"` | `AppError::PermissionDenied` | error classifier |
| `parse_plist_invalid` | `&[]` empty bytes | `Err(AppError::Internal)` | parse error |

---

## Implementation Notes

### Files modified

- `crates/disk/Cargo.toml` — added `plist = "1"` under `[target.'cfg(target_os = "macos")'.dependencies]` with DEPENDENCY JUSTIFICATION comment
- `crates/disk/src/macos/enumerate.rs` — full implementation replacing the P0 stub
- `src-tauri/src/commands/mod.rs` — replaced `greet` with `get_disks`
- `src-tauri/src/lib.rs` — updated `invoke_handler` to register `get_disks`
- `docs/agent-planning/05-p1-disk-enum-macos.md` — this planning doc

### Deviations from plan

1. `parse_disk_info_from_dict` returns `DiskInfo` (not `Result<DiskInfo, AppError>`) because the function is infallible — `clippy::unnecessary_wraps` (pedantic) correctly flagged the original `Result` return. The caller `disk_info_for` wraps it in `Ok()`.

2. The match arm `Err(_) => continue` was changed to `Err(_) => {}` — `clippy::needless_continue` (pedantic) flagged the redundant `continue` at the tail of a `for` loop body.

3. `DiskInfoFieldCase` test struct originally had 4 bools, triggering `clippy::struct_excessive_bools` (pedantic). Fixed by replacing the `check_encrypted: bool` field with a `DiskInfoField` enum (`Encrypted | Trim`).

4. The `# Errors` rustdoc section was removed from `classify_stderr_error` — the function returns `AppError` directly, not a `Result`.

### Test results

```
test macos::enumerate::tests::test_parse_plist_invalid ... ok
test macos::enumerate::tests::test_classify_stderr_error ... ok
test macos::enumerate::tests::test_extract_physical_disk_ids ... ok
test macos::enumerate::tests::test_detect_drive_type ... ok
test macos::enumerate::tests::test_detect_filesystem ... ok
test macos::enumerate::tests::test_disk_info_fields ... ok

test result: ok. 6 passed; 0 failed
```

All 16 cases from the test matrix are covered across 6 test functions (grouped by the function under test). `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` both pass with 0 issues.

### Task completion checklist

- [x] `cargo clippy` — 0 warnings
- [x] `cargo fmt` — clean
- [x] Cognitive complexity ≤ 15 on all new functions (max ~5 in `detect_filesystem`)
- [x] Unit tests written (table-driven); 16 cases across 6 test functions
- [x] No `unwrap()`/`expect()` without `// JUSTIFIED:` (test `expect()` calls have meaningful messages)
- [x] All `unsafe` blocks have `// SAFETY:` (no unsafe blocks in this task)
- [x] Planning doc updated with Implementation Notes

## Open Questions / TPM Queries

_None — no ambiguity; strategy is clearly specified._

---

## 🟢 QA Sign-off

**Date:** 2026-06-27  
**QA Agent:** 🟢  
**Result:** APPROVED

**Verified:**
- `cargo test --workspace`: 12 tests total (6 disk enumerate + 2 core + 4 windows/permissions), 0 failures
- `cargo clippy`: 0 warnings
- `cargo fmt`: clean
- Code audit: no unwrap/expect violations, all safe fallbacks justified
- Test coverage: all 16 cases from test matrix verified
  * extract_physical_disk_ids: 2 cases
  * detect_filesystem: 5 cases
  * detect_drive_type: 5 cases
  * disk_info_fields: 2 cases
  * classify_stderr_error: 1 case
  * parse_plist: 1 case
- Edge cases: all 7 edge cases from planning doc verified and handled
- Planning doc: Implementation Notes and test results complete

**P1-T01 status → DONE**

## Security Sign-off

_Not required for this task — no unsafe code, no entitlement changes._
