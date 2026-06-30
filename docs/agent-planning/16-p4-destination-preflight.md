# P4-T01 — Destination Picker & Pre-flight

**Task ID:** P4-T01
**Phase:** 4 — File Recovery
**Owner:** Developer (🔵)
**Status:** QA Review
**Security Gate:** No (no parsing of untrusted disk data; `unsafe` limited to two
documented platform syscalls — `statfs` / `GetDiskFreeSpaceExW`)

---

## Overview

Before any recovery writes a byte, the destination must be validated. This task
adds the native folder picker and the four pre-flight checks that gate the
recovery engine (P4-T02): not the same disk, enough space, writable, source
still readable.

## Scope

- `pick_destination_folder() -> Result<Option<String>, AppError>` — native picker
- `preflight_recovery(entries, source, dest_path) -> Result<PreflightResult, AppError>`
- Pure decision logic `preflight::evaluate` (recovery crate) — fully table-tested
- Platform fact gathering `fsinfo` (disk crate): free space + backing device + writability
- `PreflightError` made internally tagged (`{ "kind": ... }`) to match `types.ts`

## Out of Scope

- The recovery write itself (P4-T02)
- Any UI (P4-T03) — this is backend + IPC only
- Per-physical-disk mapping on Windows (drive-letter proxy used; heavier IOCTL deferred)

## Dependencies

- Blocked by: P3-T01 (DiskInfo + serial field; reuses BlockReader path convention)

---

## [DEV] Plan

Two layers, mirroring the P3-T01 `BlockProbe` split so the decision is testable
without disks:

1. **Pure** — `crates/recovery/src/preflight.rs`
   - `PreflightFacts { same_disk, available_bytes, dest_writable, source_readable }`
   - `required_bytes(entries) -> u64` = Σ`size_bytes` + 10% (integer `n + n/10`)
   - `evaluate(required, facts) -> PreflightResult` — facts → `Vec<PreflightError>`

2. **Platform** — `crates/disk/src/fsinfo.rs`
   - `dest_info(path) -> DestInfo { available_bytes, device }`
     - macOS: `statfs(2)` → `f_bavail * f_bsize`, `f_mntfromname` → whole-disk id
     - Windows: `GetDiskFreeSpaceExW` + drive-letter device
   - `is_writable(dir)` — real probe write + cleanup (pure std)
   - `normalize_disk_id` / `same_disk` — `/dev/rdisk3s1s1` → `disk3` (pure, tested)

3. **IPC** — `src-tauri/src/commands/recovery.rs`
   - `pick_destination_folder` via `tauri-plugin-dialog` (Rust-only; callback→oneshot)
   - `preflight_recovery` gathers facts on `spawn_blocking`, then `evaluate`
   - `source_readable(disk_id)` = can the raw device be opened read-only?

## Edge Cases

- Σ size overflow → `saturating_add` caps at `u64::MAX` (test)
- Destination on same physical disk as source → `SameDisk` (slice vs whole-disk normalized)
- Read-only / full / missing destination → `is_writable` false → `DestinationNotWritable`
- Source disconnected or no Full Disk Access → `source_readable` false → `SourceNotReadable`
- User cancels picker → `Ok(None)` (not an error)
- Path with NUL byte → `AppError::Internal` (CString guard)

## Test Plan

| Case | Input | Expected | Branch |
|------|-------|----------|--------|
| required_buffer | one 1000-byte entry | 1100 | Σ + 10% |
| required_sum | 1000 + 2000 | 3300 | fold |
| required_overflow | 2×u64::MAX | u64::MAX | saturating |
| all_pass | good facts | ok=true, no errors | happy |
| same_disk | same_disk=true | SameDisk present | check 1 |
| insufficient | avail<required | InsufficientSpace{required,available} | check 2 |
| read_only_dest | dest_writable=false | DestinationNotWritable | check 3 |
| source_gone | source_readable=false | SourceNotReadable | check 4 |
| multi_fail | all bad | 4 errors, ok=false | independence |
| normalize_* | dev/raw/slice spellings | `diskN` | path reduction |
| same_disk_spellings | slice vs whole | match / no-match | comparison |
| is_writable_temp | temp dir | true | probe write |
| is_writable_missing | nonexistent dir | false | probe fail |

---

## Implementation Notes

- **`PreflightError` serde tag.** Was externally tagged by default
  (`{"SameDisk":null}` / `{"InsufficientSpace":{…}}`), which does **not** match
  the `{ kind: 'SameDisk' }` discriminated union in `src/lib/types.ts`. Added
  `#[serde(tag = "kind")]` so the existing TS type is correct with no FE change.
- **`required_bytes` uses `size_bytes`, not `estimated_recoverable_bytes`** — see
  DECISION-018. `size_bytes ≥ estimated`, so the space reservation is always
  safe and the command needs no `ProbabilityReport` argument.
- **Same-disk via BSD device id, not serial** (DECISION-018). `DiskInfo.serial`
  is frequently `None` on macOS; `statfs` `f_mntfromname` gives a reliable
  whole-disk id that normalizes against `DiskInfo.id`.
- **`pick_destination_folder` returns `Option<String>`** (DECISION-018) so user
  cancel is a value, not an `Err`. Dialog plugin is invoked only from Rust → no
  frontend dialog capability added to `capabilities/default.json`.
- **`unsafe`**: two blocks, both with `// SAFETY:` — `statfs` (macOS),
  `GetDiskFreeSpaceExW` (Windows). No untrusted-input parsing.
- Verified: `cargo clippy --workspace --all-targets -D warnings` clean;
  `cargo fmt --check` clean; `cargo test --workspace` green (recovery 10, disk 45).

## Task Completion Checklist

- [x] `cargo clippy` — 0 warnings
- [x] `cargo fmt` — clean
- [x] Cognitive complexity ≤ 15 on all new functions
- [x] Unit tests written (table-driven); recovery + disk green
- [ ] `make smoke` — N/A (no UI/build surface in this task)
- [x] No `unwrap()`/`expect()` without `// JUSTIFIED:`
- [x] All `unsafe` blocks have `// SAFETY:`
- [x] Planning doc updated with `## Implementation Notes`
- [ ] 🟢 QA sign-off appended
- [ ] 🔴 Security sign-off — N/A (no security gate on this task)

## Open Questions / TPM Queries

None outstanding. Three implementation refinements were resolved and logged as
DECISION-018 rather than blocking queries (all narrow the spec conservatively).

---

## 🟢 QA Sign-off

**Date:** 2026-06-30 · **Agent:** QA

- `cargo test --workspace` — green. Pure `evaluate`/`required_bytes` covered by
  table tests (all 4 checks individually + combined + overflow); `fsinfo` pure
  helpers (`normalize_disk_id`, `same_disk`, `is_writable`) covered (13 cases).
- `cargo clippy --workspace --all-targets -D warnings` — 0 warnings.
- `cargo fmt --all --check` — clean.
- Both `unsafe` blocks carry `// SAFETY:`; no `unwrap`/`expect` outside tests.
- No UI surface → `make smoke` N/A.
- **Verdict: PASS.** No security gate on this task.

The platform `dest_info` (statfs / `GetDiskFreeSpaceExW`) is not unit-tested (it
needs a real volume); it is a thin syscall wrapper and the decision logic it
feeds is fully tested. Acceptable for sign-off; flagged for the P5 on-device pass.
