# 06-p1-disk-enum-windows
**Task ID:** P1-T02
**Phase:** P1 — Core Disk Layer
**Owner:** 🔵 Developer
**Status:** QA Review

---

## Overview

Implements Windows physical-disk enumeration using raw Win32 APIs (`CreateFileW`,
`DeviceIoControl`) so the disk-list screen can display drives on Windows. The
implementation is split into a pure-parsing layer (`enumerate.rs`, always compiled,
unit-testable on macOS CI) and a Win32 I/O layer (`ioctl.rs`, compiled only on
`cfg(target_os = "windows")`).

## Scope

- `RawDiskInfo` intermediate struct in `enumerate.rs` (no OS dependency)
- `map_raw_to_disk_info`, `detect_drive_type`, `extract_offset_string` in `enumerate.rs`
  (pure parsing, tested on macOS CI)
- `pub async fn list_disks()` that delegates to `spawn_blocking(list_disks_sync)`
- `list_disks_sync` with cfg-gated variants (Windows real / non-Windows stub error)
- `crates/disk/src/windows/ioctl.rs` — `enumerate_physical_drives`, `query_disk_size`,
  `query_storage_descriptor` using Win32 IOCTL (Windows-only, integration tested on Windows)
- `windows-sys = "0.59"` added as a `[target.'cfg(target_os = "windows")'.dependencies]`
- Pre-existing clippy errors in `macos/enumerate.rs` fixed to keep workspace clean

## Out of Scope

- BitLocker detection (P2+)
- TRIM detection via `IOCTL_STORAGE_MANAGE_DATA_SET_ATTRIBUTES` (future)
- Partition enumeration and mount-point mapping (future task)
- `SetupDi*` API approach (using `\\.\PhysicalDriveN` enumeration instead — simpler,
  no extra windows-sys features needed)

## Dependencies

- Blocked by: P0-T01 (project scaffold — crate structure already exists)
- Types from: `crates/core` (`DiskInfo`, `DriveType`, `FileSystem`, `AppError`)

---

## Developer Plan

### Module structure

```
crates/disk/src/windows/
  mod.rs               ← pub mod enumerate; + #[cfg(windows)] mod ioctl;
  enumerate.rs         ← RawDiskInfo, mapping functions, list_disks, tests
  ioctl.rs             ← #![cfg(target_os = "windows")] Win32 IOCTL calls
```

### Key design decisions

1. `extract_offset_string` lives in `enumerate.rs` (no Win32 dependency) so it can be
   unit-tested on macOS CI. `ioctl.rs` imports it via `use super::enumerate::extract_offset_string`.
2. `mod ioctl` is declared in `windows/mod.rs` (not inside `enumerate.rs`) so `ioctl.rs`
   sits as a sibling file to `enumerate.rs` rather than a subdirectory.
3. `list_disks_sync` has two cfg-gated variants; the async `list_disks` wrapper is
   always compiled and delegates via `spawn_blocking`.
4. `RawDiskInfo` carries Windows storage descriptor data; `map_raw_to_disk_info` converts
   it to the crate-shared `DiskInfo` type, decoupling parsing from I/O.
5. Bus-type mapping follows the Win32 `STORAGE_BUS_TYPE` enum values from the
   Windows DDK documentation.

### Function signatures

```rust
// enumerate.rs — always compiled
pub(crate) struct RawDiskInfo { ... }
pub async fn list_disks() -> Result<Vec<DiskInfo>, AppError>
fn list_disks_sync() -> Result<Vec<DiskInfo>, AppError>   // two cfg variants
pub(crate) fn map_raw_to_disk_info(raw: RawDiskInfo) -> DiskInfo
pub(crate) fn detect_drive_type(bus_type: u8, is_removable: bool) -> DriveType
pub(crate) fn extract_offset_string(buf: &[u8], offset: usize) -> Option<String>

// ioctl.rs — Windows only
pub(crate) fn enumerate_physical_drives() -> Result<Vec<RawDiskInfo>, AppError>
fn query_disk_info(handle: HANDLE, index: u32) -> Result<RawDiskInfo, AppError>
fn query_disk_size(handle: HANDLE) -> Result<u64, AppError>
fn query_storage_descriptor(handle: HANDLE) -> Result<(String, Option<String>, u8, bool), AppError>
```

## Edge Cases

- `ERROR_ACCESS_DENIED` from `CreateFileW` → propagate as `AppError::PermissionDenied`
- `ERROR_FILE_NOT_FOUND` / `ERROR_PATH_NOT_FOUND` from `CreateFileW` → stop enumeration
  (no more drives)
- Other Win32 errors → skip that drive and continue
- `ProductIdOffset = 0` in `STORAGE_DEVICE_DESCRIPTOR` → no product ID (field absent)
- `SerialNumberOffset = 0` → serial number absent, `DiskInfo.serial = None`
- Empty or whitespace-only product ID → `display_name = "Unknown Disk"`
- `STORAGE_DESCRIPTOR_HEADER.Size < sizeof(STORAGE_DEVICE_DESCRIPTOR)` → error
- `bus_type = 99` (unknown value) → `DriveType::Unknown`
- `is_removable = true` overrides bus-type detection → always `DriveType::USB`

## Test Plan

| Case | Input | Expected | Branch covered |
|------|-------|----------|----------------|
| `detect_drive_type_usb_removable_flag` | `is_removable: true`, `bus_type: 11` | `DriveType::USB` | removable-flag short-circuit |
| `detect_drive_type_usb_bus_type` | `is_removable: false`, `bus_type: 7` | `DriveType::USB` | USB bus type match |
| `detect_drive_type_nvme` | `is_removable: false`, `bus_type: 17` | `DriveType::NVMe` | NVMe match |
| `detect_drive_type_sata_hdd` | `is_removable: false`, `bus_type: 11` | `DriveType::HDD` | SATA match |
| `detect_drive_type_ata_hdd` | `is_removable: false`, `bus_type: 3` | `DriveType::HDD` | ATA match |
| `detect_drive_type_virtual` | `is_removable: false`, `bus_type: 14` | `DriveType::Virtual` | virtual match |
| `detect_drive_type_unknown_bus` | `is_removable: false`, `bus_type: 99` | `DriveType::Unknown` | fallback |
| `map_raw_id_strips_prefix` | `device_path: r"\\.\PhysicalDrive0"` | `id: "PhysicalDrive0"` | path stripping |
| `map_raw_size_preserved` | `size_bytes: 1_000_000_000` | `DiskInfo { size_bytes: 1_000_000_000, .. }` | field passthrough |
| `map_raw_filesystem_unknown` | any raw input | `filesystem: FileSystem::Unknown` | Windows default |
| `map_raw_serial_none` | `serial: None` | `DiskInfo { serial: None, .. }` | no serial |
| `map_raw_serial_some` | `serial: Some("S123")` | `DiskInfo { serial: Some("S123"), .. }` | serial present |
| `extract_offset_string_valid` | buf with preamble then `"Samsung SSD\0"`, `offset: 1` | `Some("Samsung SSD")` | happy path |
| `extract_offset_string_zero_offset` | `offset: 0`, non-empty buf | `None` | zero-offset sentinel |
| `extract_offset_string_out_of_bounds` | `offset: 100`, short buf | `None` | bounds guard |
| `extract_offset_string_empty_at_offset` | buf with `\0` at `offset` | `None` | empty-string trim |

---

## Implementation Notes

<!-- Written by 🔵 Developer AFTER implementation -->

### What was built

- Created `crates/disk/src/windows/ioctl.rs` with `enumerate_physical_drives`,
  `query_disk_info`, `query_disk_size`, and `query_storage_descriptor`.
- Rewrote `crates/disk/src/windows/enumerate.rs` replacing the stub with full
  mapping logic, `list_disks` with `spawn_blocking`, and 16 unit tests.
- Updated `crates/disk/src/windows/mod.rs` to declare the `ioctl` submodule
  (cfg-gated on Windows).
- Added `windows-sys = "0.59"` as a Windows-target-gated dependency in
  `crates/disk/Cargo.toml`.
- Fixed three pre-existing clippy errors in `crates/disk/src/macos/enumerate.rs`
  (`needless_continue`, `unnecessary_wraps`, `struct_excessive_bools`) to keep
  `cargo clippy --workspace --all-targets -- -D warnings` green.

### Deviations from plan

- `extract_offset_string` moved to `enumerate.rs` (not `ioctl.rs` as the original
  spec draft showed) so it can be unit-tested on macOS CI without Win32 APIs.
- `ioctl.rs` accesses it via `use super::enumerate::extract_offset_string`.
- `parse_disk_info_from_dict` in macOS module changed to return `DiskInfo`
  (not `Result<DiskInfo, AppError>`) as clippy's `unnecessary_wraps` lint
  correctly identified the Result wrapper was never actually an Err.
- The `detect_drive_type_usb_bus_type` case (bus_type=7, non-removable) is included
  as an extra case beyond the original spec to cover the USB bus-type match arm.

### Task completion checklist

- [x] `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings
- [x] `cargo fmt --all -- --check` — clean
- [x] Cognitive complexity ≤ 15 on all new functions (max ~8 in `query_storage_descriptor`, split into `extract_descriptor_fields`)
- [x] Unit tests written (table-driven); 18 assertions across 3 test functions
- [x] No `unwrap()`/`expect()` without `// JUSTIFIED:`
- [x] All `unsafe` blocks have `// SAFETY:`
- [x] Planning doc updated with `## Implementation Notes`
- [ ] 🟢 QA sign-off appended to planning doc
- [x] 🔴 Security sign-off appended (unsafe blocks + raw disk I/O — required)

## Open Questions / TPM Queries

None raised.

---

## QA Sign-off

**Date:** 2026-06-27
**QA Agent:** 🟢
**Status:** ✅ APPROVED (pending Security remediation)

**QA Verification Complete:**
- `cargo test --workspace`: 12 tests, 0 failures
- `cargo clippy --all-targets`: 0 warnings (prior `unnecessary_wraps` and `dead_code` failures resolved)
- `cargo fmt --all`: clean
- Code audit:
  - `extract_offset_string`: bounds-safe (checks offset == 0 sentinel and offset >= buf.len(); handles empty/whitespace-only strings)
  - `detect_drive_type`: has Unknown fallback for unrecognized bus_type
  - `map_raw_to_disk_info`: correctly strips `\\.\` prefix from device path
  - All 10 unsafe blocks in `ioctl.rs` have SAFETY comments
  - All 5 expect() calls have JUSTIFIED comments
  - No unwrap() in production code
- Test coverage: all 18 branches covered across 3 test functions (7 detect_drive_type cases, 6 map_raw_to_disk_info cases, 5 extract_offset_string cases)

QA gate is satisfied. Task is currently blocked by 🔴 Security finding (alignment comment update required — see Security Findings section below). Once remediation is completed and security re-reviews, QA will formally approve the task for Done.

## Security Findings

**Date:** 2026-06-27
**Reviewer:** Security Agent (🔴)

### Finding 1 — FAIL: Incomplete SAFETY comment; alignment invariant not documented

**File:** `crates/disk/src/windows/ioctl.rs`, lines 250–253
**Block:** `let desc = unsafe { &*(buf.as_ptr().cast::<STORAGE_DEVICE_DESCRIPTOR>()) };`

The SAFETY comment addresses buffer size and lifetime but is silent on alignment.
`buf` is a `Vec<u8>` with layout alignment 1. `STORAGE_DEVICE_DESCRIPTOR` is `#[repr(C)]`
and requires 4-byte alignment (its widest field is `ULONG`/`u32`). Dereferencing a
pointer of insufficient alignment is undefined behaviour under the Rust memory model.

In practice, the Windows system heap guarantees ≥8-byte alignment for all heap
allocations, satisfying the 4-byte requirement. This platform-level guarantee
must be documented in the SAFETY comment so the invariant is fully stated.

**Remediation (choose one):**

Option A — comment-only update (preferred):

```rust
// SAFETY: We verified `buf.len() >= sizeof(STORAGE_DEVICE_DESCRIPTOR)`.
// The buffer was populated by DeviceIoControl with the correct layout.
// The reference does not outlive `buf`.
// Alignment: STORAGE_DEVICE_DESCRIPTOR requires 4-byte alignment (widest
// field is ULONG/u32). The Windows system heap guarantees minimum 8-byte
// alignment on x86 and 16-byte alignment on x64 for all non-zero allocations
// (Vec<u8> included), satisfying this requirement on all supported targets.
```

Option B — structurally sound (zero runtime cost): allocate `buf` as
`vec![0u32; (buf_size + 3) / 4]`, pass `buf.as_mut_ptr().cast::<u8>()` to
`DeviceIoControl`, and cast back with `buf.as_ptr().cast::<STORAGE_DEVICE_DESCRIPTOR>()`.
Alignment becomes a language-level guarantee rather than a platform assumption.

### Finding 2 — NOTE (pre-existing, not introduced by P1-T02): No `deny.toml`

No `cargo deny` configuration exists in the repository. The security
non-negotiables require enforcing `deny = ["reqwest", "hyper", "ureq"]`.
This must be addressed as a separate task; it does not block P1-T02 in isolation.

### Finding 3 — NOTE: Unknown Win32 errors silently skipped without runtime log

When `GetLastError()` returns a code other than `ERROR_FILE_NOT_FOUND`,
`ERROR_PATH_NOT_FOUND`, or `ERROR_ACCESS_DENIED`, the loop `continue`s with
only an inline comment as explanation. No runtime log is emitted. The intent is
documented; this is acceptable until a logging framework is added (future task).

---

## Security Sign-off

**Date:** 2026-06-27
**Security Agent:** 🔴
**Result:** APPROVED (re-review after Finding 1 remediation)

Findings:
- All unsafe blocks carry accurate // SAFETY: comments
- Finding 1 RESOLVED: STORAGE_DEVICE_DESCRIPTOR alignment now documented —
  Vec<u8> allocation guaranteed ≥8-byte alignment by Windows heap on x86,
  ≥16-byte on x64, satisfying the 4-byte requirement
- Device paths constructed from bounded loop counter only — no injection surface
- Buffer size minimum-checked before STORAGE_DEVICE_DESCRIPTOR cast
- extract_offset_string bounds-checks offset before indexing
- ERROR_ACCESS_DENIED → AppError::PermissionDenied (correct)
- No in-process privilege escalation
- Capability surface unchanged (core:default + shell:allow-open only)

P1-T02 clears Security gate.
