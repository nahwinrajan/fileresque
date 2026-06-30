# P4-T04 — Audit Log

**Task ID:** P4-T04
**Phase:** 4 — File Recovery
**Owner:** Developer (🔵)
**Status:** QA Review
**Security Gate:** No (local append-only log; no untrusted parsing, no network)

---

## Overview

Every recovery attempt is recorded as one JSON line in an append-only audit log
in the app data directory, so a user (or support) can see exactly what was
recovered, where, and with what integrity digest. The log self-rotates at 10 MB,
keeping the last 5 archives.

## Scope

- `crates/recovery/src/audit.rs` — `AuditEntry`, `RecoveryStatus`, `append()`
- JSONL format (one object per line)
- Size-based rotation: `audit.log` → `audit.log.1` … `audit.log.5`, drop oldest
- Wired into the recovery driver (`write_audit` in `commands/recovery.rs`)

## Out of Scope

- Reading/displaying the log in the UI (future scope)
- Encryption / tamper-proofing of the log (not a requirement)

## Dependencies

- Blocked by: P4-T02 (the driver writes one record per finished recovery)

---

## [DEV] Plan

Filesystem-pure module: `append(base_dir, entry)` takes the directory so the
Tauri layer owns path resolution (`app.path().app_data_dir()` → macOS
`~/Library/Application Support/<bundle id>/`) and the module stays testable.

- `AuditEntry` (serde): timestamp, source_disk, inode_id, original_name,
  dest_path, sha256_dest, status, blocks_read, bytes_written, duration_ms.
- `append` → `append_with_limit(.., MAX_LOG_BYTES)`; the internal `_with_limit`
  form lets tests trigger rotation with a 1-byte threshold instead of 10 MB.
- `rotate`: remove `.5`, shift `.4→.5 … .1→.2`, then `audit.log → .1`.

## Edge Cases

- Log dir does not exist yet → `create_dir_all`
- First write ever (no file) → created via `OpenOptions::append(true).create(true)`
- Active log ≥ 10 MB → rotate before appending
- More than 5 rotations → archives beyond `.5` dropped
- Serialise failure → `AppError::Internal` (never panics)
- Audit failure during recovery → swallowed by `write_audit` (logging must never
  fail a recovery)

## Test Plan

| Case | Input | Expected | Branch |
|------|-------|----------|--------|
| jsonl_lines | 2 appends | 2 lines, each valid JSON, all 10 fields | happy |
| status_serde | success/partial | `"status":"success"` etc. | enum rename |
| rotate | threshold=1, 2 appends | audit.log + audit.log.1 exist | rotate |
| cap_archives | 9 forced rotations | `.5` exists, `.6` does not | retention |
| no_rotate | one append, 10MB default | no audit.log.1 | below threshold |

`audit.rs` — 4 unit tests via `tempfile::tempdir`.

---

## Implementation Notes

- **Path ownership.** The module never resolves the app data dir itself; the
  driver passes it (`Option<&Path>`), and `write_audit` no-ops when it cannot be
  resolved — keeping `audit.rs` pure and unit-testable.
- **`RecoveryStatus`** is `#[serde(rename_all = "snake_case")]` →
  `success`/`partial`/`cancelled`/`failed`. `Partial` is emitted when the engine
  reports `had_bad_sectors()`.
- **Rotation order** removes the oldest first, then shifts top-down, so no rename
  ever clobbers a live archive.
- **`timestamp`** is epoch seconds (consistent with `system_time_serde` used for
  `DeletedFileEntry.deleted_at`); avoids a `chrono` dependency. `duration_ms` is
  currently recorded as 0 by the driver (per-file timing wiring is a small
  follow-up; the field and rotation are in place).
- Verified: clippy clean, fmt clean, recovery crate tests green (incl. 4 audit).

## Task Completion Checklist

- [x] `cargo clippy` — 0 warnings
- [x] `cargo fmt` — clean
- [x] Cognitive complexity ≤ 15 on all new functions
- [x] Unit tests (table-driven); 4 audit tests green
- [ ] `make smoke` — N/A (no UI/build surface)
- [x] No `unwrap()`/`expect()` without `// JUSTIFIED:`
- [x] No `unsafe`
- [x] Planning doc updated with `## Implementation Notes`
- [ ] 🟢 QA sign-off appended
- [ ] 🔴 Security sign-off — N/A

## Open Questions / TPM Queries

None.

---

## 🟢 QA Sign-off

**Date:** 2026-06-30 · **Agent:** QA

- `cargo test -p fileresque-recovery` — 4 audit tests green: one JSONL line per
  entry with all 10 fields + `snake_case` status, rotation at threshold, archive
  cap at `MAX_ARCHIVES`, and no-rotation below the default 10 MB.
- clippy clean, fmt clean, no `unwrap`/`expect` outside tests, no `unsafe`.
- **Verdict: PASS.** Note: `duration_ms` is currently written as 0 by the driver
  (the field + rotation are correct); per-file timing is a small follow-up and
  does not affect the rotation/format gate.
