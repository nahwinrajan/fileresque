# P4-T02 — Recovery Engine

**Task ID:** P4-T02
**Phase:** 4 — File Recovery
**Owner:** Developer (🔵)
**Status:** QA Review · **Security Gate: YES (🔴 required)**

---

## Overview

The core of Phase 4: read a deleted file's block extents off the raw source
device and write them to the validated destination as a real file. Crash-safe
(`.partial` → atomic rename), bad-sector tolerant, cancellable, and — the focus
of the security gate — every filename derived from raw disk metadata is
sanitised before it touches the filesystem.

## Scope

- `crates/recovery/src/engine.rs` — `recover()` over the `ExtentReader` trait
- Filename sanitisation (`sanitize_filename`) — traversal/separator/device-name safe
- Magic-byte extension inference (`infer_extension`) — JPEG/PNG/PDF/ZIP/MP4/MOV
- SHA-256 of written content (returned + audited)
- `.partial` temp write + `PartialGuard` (RAII cleanup) + atomic `fs::rename`
- Bad-sector handling: zero-fill + count + continue
- Cancellation via `should_cancel` closure
- IPC driver (`recover_files` / `cancel_recovery`) + `BlockReader`→`ExtentReader` adapter
- Throttled `recovery:progress` / `recovery:file_complete` / `recovery:complete` events

## Out of Scope

- Recovery UI (P4-T03)
- Windows NTFS recovery path (adapter returns `UnsupportedFilesystem`; macOS APFS wired)
- `SIGBUS`/`mmap` handling — reads go through `read(2)` via `BlockReader`, which
  returns `EIO` as an `Err` (treated as bad sector); no memory-mapped reads used

## Dependencies

- Blocked by: P4-T01 (destination validated before this runs)

---

## [DEV] Plan

Same trait-split pattern as P3-T01 so the engine is unit-testable with no disk:

- **`trait ExtentReader { block_size(); read_block(addr) }`** — the only I/O surface.
- **`recover(req, reader, progress, should_cancel) -> RecoveryOutcome`** — pure
  orchestration: open `.partial`, `stream_extents`, flush, choose final name,
  `rename`, commit guard.
- `stream_extents` → per block: cancel-check → `read_or_zero` → `write_capped`
  (caps at `size_bytes`, updates SHA + header) → progress tick.
- Final name: `sanitize_filename(entry.name)` if usable, else
  `recovered_<sha8>.<inferred_ext>`; `unique_path` dedupes collisions.

IPC driver (`src-tauri/src/commands/recovery.rs`): `recover_files` spawns a task
that runs each entry via `spawn_blocking` (DECISION-005), emits events, writes an
audit record (P4-T04). `cancel_recovery` flips an `Arc<AtomicBool>` the engine
polls. Adapter `DeviceExtentReaderImpl` wraps `BlockReader`, fixing block size
from the APFS superblock.

## Edge Cases

- **Path traversal** in `entry.name` (`../../etc/passwd`) → reduced to basename
- Embedded separators / illegal chars / Windows device names (`CON`) → sanitised/rejected
- Overlong name (>200 bytes) → truncated on a char boundary
- Bad sector mid-file → zero-filled block, offsets preserved, `Partial` status
- Cancel mid-file → `.partial` removed by `PartialGuard::drop`, `Cancelled` status
- Final name already exists → `_1`, `_2`, … suffix
- `size_bytes == 0` with extents → write full blocks (no cap); else cap at size
- Panic during write → guard still removes `.partial`

## Test Plan

| Case | Input | Expected | Branch |
|------|-------|----------|--------|
| happy | 2 blocks, name set | file written, renamed, sha set, 0 skipped | main |
| truncate | 8-byte block, size 5 | 5 bytes on disk | cap |
| bad_sector | middle block errors | zero-filled, blocks_skipped=1, Partial | read_or_zero |
| cancel | should_cancel=true | Err(Cancelled), no files left | cancel + guard |
| dedupe | f.bin exists | f_1.bin | unique_path |
| generated_name | name=None, JPEG magic | recovered_*.jpg | final_file_name |
| sanitize_* | traversal/sep/device/empty | basename / None | security boundary |
| sanitize_truncate | 500 chars | ≤200 bytes | truncate_bytes |
| infer_extension | 7 magic headers | jpg/png/pdf/zip/mp4/mov/bin | inference |

All in `engine.rs` (13 unit tests via in-memory `MemReader` with bad-sector injection).

---

## Implementation Notes

- **SEC boundary is `sanitize_filename`** — `rsplit(['/','\\'])` keeps only the
  final component (defeats traversal), strips control chars, maps illegal chars
  to `_`, trims dots/space, rejects `.`/`..`/empty/reserved device names, caps at
  200 bytes. Returns `None` → caller uses a generated name. 10 table cases incl.
  `../../etc/passwd`, `evil\..\boot.ini`, `CON`, `nul.txt`.
- **No `unsafe` in the engine.** The only `unsafe` in Phase 4 is the two
  `statfs`/`GetDiskFreeSpaceExW` calls in P4-T01's `fsinfo`, already documented.
- **Atomicity**: write to a process+inode+nanos `.partial`, then `fs::rename`
  (atomic within a volume — and the destination *is* one volume by preflight).
- **`PartialGuard`** removes the temp file on every non-commit drop path
  (error, cancel, panic). Verified by `cancel_aborts_and_cleans_partial` and the
  leftover-count assertion in the happy-path test.
- **SHA-256** hashes exactly the bytes written (post-cap), so the digest matches
  the on-disk file. `hex_lower` avoids a hex-crate dependency.
- **Progress throttling**: emit on block 1 and every `PROGRESS_STRIDE` (64)
  blocks to avoid flooding IPC on multi-GB files.
- Verified: `cargo clippy --workspace --all-targets -D warnings` clean;
  `cargo fmt --check` clean; `cargo test --workspace` green (recovery 23).

## Task Completion Checklist

- [x] `cargo clippy` — 0 warnings
- [x] `cargo fmt` — clean
- [x] Cognitive complexity ≤ 15 on all new functions (clippy-enforced, clean)
- [x] Unit tests (table-driven, mock disk fixture); recovery crate 23 green
- [ ] `make smoke` — N/A (no UI in this task; UI is P4-T03)
- [x] No `unwrap()`/`expect()` without `// JUSTIFIED:`
- [x] No `unsafe` in this task (engine has none)
- [x] Planning doc updated with `## Implementation Notes`
- [ ] 🟢 QA sign-off appended
- [ ] 🔴 **Security sign-off appended (REQUIRED — sanitisation + raw-device read)**

## Open Questions / TPM Queries

None. `recovered_<hash>` uses the full-file SHA-256 truncated to 8 hex chars
rather than "sha256_of_first_block"; the digest is already computed and is a
stronger unique id. Logged here as a minor, non-blocking deviation.

---

## 🟢 QA Sign-off

**Date:** 2026-06-30 · **Agent:** QA

- `cargo test -p fileresque-recovery` — 23 green, incl. the engine's 13: happy
  path + atomic rename, size truncation, bad-sector zero-fill, cancel cleanup,
  collision dedupe, generated-name + magic byte, the 10-case sanitisation table,
  overlong-name truncation, and the 7-case extension table.
- `cargo clippy --workspace --all-targets -D warnings` — 0 warnings (cognitive
  complexity ≤ 15 enforced and clean across all new fns).
- `cargo fmt --all --check` — clean. No `unwrap`/`expect` outside tests without
  `// JUSTIFIED:`; no `unsafe` in the engine.
- **Verdict: PASS** (unit/lint gate). Recovery against a real APFS device is an
  on-device check tracked for P5; the mock-fixture coverage satisfies the
  P4-T02 QA gate as written ("unit test with mock disk fixture").

---

## 🔴 Security Sign-off

**Date:** 2026-06-30 · **Agent:** Security (veto holder)

Reviewed against the gate's two concerns — path traversal from disk metadata,
and raw-device handling.

- **Filename sanitisation (primary concern).** `sanitize_filename` keeps only the
  final path component via `rsplit(['/','\\'])`, so embedded traversal
  (`../../etc/passwd`, `evil\..\boot.ini`) collapses to a basename; strips
  control chars/NUL; maps `:*?"<>|` to `_`; trims dots/whitespace; rejects
  `.`/`..`/empty and Windows reserved device names; caps at 200 bytes on a UTF-8
  boundary. On reject → `None`, and the engine falls back to a generated
  `recovered_<hash>.<ext>` name. The final path is always `dest_dir.join(<single
  component>)` — the destination cannot be escaped. Traversal cases are asserted
  in `sanitize_filename_blocks_traversal_and_separators`.
- **No `unsafe` in the engine.** Source reads go through `BlockReader`
  (`read(2)`/`seek`), not `mmap`, so a bad sector returns `EIO` as `Err` and is
  contained (zero-filled, counted) rather than raising `SIGBUS`.
- **Write safety.** Output is confined to the (preflight-validated) destination
  via a `.partial` temp file; `PartialGuard::drop` removes it on every
  error/cancel/panic path; only a complete file is `rename`d into place. No
  clobber: `unique_path` suffixes collisions.
- **Source opened read-only** (`File::open`); recovery never writes to the source.

**Verdict: APPROVED.** No over-permission introduced; the IPC surface adds only
`recover_files`/`cancel_recovery` (and P4-T01's two), all behind the existing
deny-by-default capability set. `tauri-plugin-dialog` is invoked from Rust only.
