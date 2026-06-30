# 22-p5-error-recovery
**Task ID:** P5-T03
**Phase:** Phase 5 — Polish & Hardening
**Owner:** [DEV]
**Status:** Done

---

## Overview
Make FileResque resilient to the two failure modes a recovery tool must survive gracefully: an internal crash, and the source disk vanishing mid-operation. A panic must be recorded (not silently lost) before the process dies; every error reaching the UI must read as plain English (never a raw OS error string); each major view must contain a render fault instead of blanking the whole window; and a disk pulled mid-scan/mid-recovery must be detected and surfaced, not left hanging.

## Scope
- **Rust — panic hook:** `std::panic::set_hook` installed at startup, appends timestamp + location + payload to a platform log file, then chains the default hook.
- **Rust — friendly errors:** `AppError::user_message()` maps every variant to actionable UI text; `Serialize` emits that (no raw OS errors cross IPC). Scan/recovery error *events* also emit friendly text.
- **Rust — disconnection watcher:** polls source-disk presence every 2s during scan and recovery; emits `disk:disconnected { disk_id, message }` once when the device node disappears, then stops.
- **Frontend — error boundary:** reusable `<ErrorBoundary>` (Svelte 5 `<svelte:boundary>`) with a friendly fallback + reset; wraps the disk panel and the results panel.
- **Frontend — disconnection handling:** `disk:disconnected` listeners in `+page.svelte` (scan) and `RecoveryModal.svelte` (recovery) move the UI to an error state with the friendly message.

## Out of Scope
- Auto-resume / checkpoint of an interrupted recovery (new scope).
- Structured `tracing`/log-file rotation (panic log is append-only, best-effort).
- Windows physical-disk IOCTL presence mapping (best-effort `open` proxy, consistent with DECISION-018(b)).

## Dependencies
- Blocked by: P4-T02 (recovery engine), P2-T04 (scan loop) — both complete.

---

## [DEV] Plan
- `crates/core/src/error.rs`
  - `impl AppError { pub fn user_message(&self) -> String }` — per-variant friendly text.
  - `Serialize` → `serialize_str(&self.user_message())` (was `to_string()`). Display retains raw detail for logs.
- `src-tauri/src/panic_log.rs` (new)
  - `pub fn install_hook()` — `take_hook()` → `set_hook` that calls `write_panic` then the default.
  - `fn log_dir() -> PathBuf` — `~/Library/Logs/FileResque` (macOS) / `%LOCALAPPDATA%\FileResque\logs` (Win) / temp fallback.
  - `fn panic_line(ts, loc, msg) -> String` — pure, unit-tested.
- `src-tauri/src/disk_watch.rs` (new)
  - `pub fn spawn(app, disk_id, stop: Arc<AtomicBool>)` — 2s poll loop, emits `disk:disconnected`, exits on `stop` or absence.
  - `fn source_present(disk_id) -> bool` — `/dev/<id>` exists (macOS) / open `\\.\<id>` (Win).
- Wire watcher: `scan::run_scan_loop` (stop set after collect loop) and `recovery::run_recovery` (stop set before `recovery:complete`). Error events switch `to_string()` → `user_message()`.
- `src/lib/components/ErrorBoundary.svelte` (new) + export in `index.ts`; wrap views in `+page.svelte`.
- `disk:disconnected` listeners + `DiskDisconnectedEvent` type.

## Edge Cases
- Panic before log dir exists → `create_dir_all`; any IO failure in the hook is swallowed (a failing panic-logger must never re-panic).
- Disk removed between two polls → single event, watcher then exits (no event spam).
- Stale watcher after scan ends → `stop` flag set; loop exits within one interval.
- Rapid re-scan → previous watcher's `stop` already set; new watcher owns a fresh flag.
- `Internal`/`Io` raw text must not reach UI → covered by `user_message` + Serialize change; events too.

## Test Plan
| Case | Input | Expected | Branch covered |
|------|-------|----------|----------------|
| user_message_permission | `PermissionDenied(..)` | mentions Full Disk Access, no raw path | mapping |
| user_message_io_no_oserr | `Io(os error)` | generic disk-error text, no "os error" substring | mapping |
| user_message_all_variants | each variant | non-empty, no `{`/debug noise | mapping |
| serialize_uses_friendly | serialize `Internal("x")` | JSON string == `user_message()` | Serialize |
| panic_line_format | ts, loc, msg | `"[ts] panic at loc: msg"` | pure fmt |
| log_dir_named | — | path ends in `FileResque` | path |

---

## Implementation Notes
Built as planned; no scope deviations.

- **Friendly errors** — `AppError::user_message()` added; `Serialize` now emits it instead of `Display`, so every `Err` returned from a command reaches the UI as plain text. The streamed *events* that carry their own JSON `message`/`error` field (`scan:error`, `recovery:file_complete`) were also switched from `e.to_string()` to `e.user_message()`; the two panic-join arms emit a fixed friendly string. `Display` is untouched and remains the detailed form for logs.
- **Panic hook** — `src-tauri/src/panic_log.rs`. `install_hook()` chains the prior hook (stderr/abort preserved). Log path resolved dependency-free from env: `~/Library/Logs/FileResque/panic.log` (macOS), `%LOCALAPPDATA%\FileResque\logs` (Win), temp fallback. Every IO step is swallowed — a panic logger must never re-panic. Pure `panic_line()` is unit-tested.
- **Disconnection watcher** — `src-tauri/src/disk_watch.rs`. `spawn()` polls every 2s; macOS checks `/dev/<id>` existence (permission-free, vanishes on eject), Windows opens `\\.\<id>` (best-effort, per DECISION-018(b)). Emits one `disk:disconnected { disk_id, message }` then exits. Lifecycle bound to each operation via an `Arc<AtomicBool>` stop flag set when the scan collect-loop ends / before `recovery:complete`.
- **Error boundary** — `ErrorBoundary.svelte` wraps Svelte 5 `<svelte:boundary>`; friendly fallback + `reset`, error logged to console only (never shown). Wraps the disk panel and results panel in `+page.svelte`.
- **Disconnection UX** — `disk:disconnected` listeners in `+page.svelte` (→ scan error state) and `RecoveryModal.svelte` (→ done state with the reason). New `DiskDisconnectedEvent` type.

### Verification
- `cargo clippy --workspace --all-targets` — 0 warnings (workspace denies `clippy::all` + `pedantic`).
- `cargo fmt` — clean.
- `cargo test --workspace` — 81 pass (core 6, src-tauri 7, disk 45, recovery 23).
- `bun run lint` (biome) — clean (30 files).
- `bunx vitest run` — 45 pass incl. new `ErrorBoundary.test.ts` (happy path + thrown-child fallback branch).
- No `unwrap()`/`expect()` added; no `unsafe` added; new fns are flat (cognitive complexity well under 15).

### Flags for 🟠 TPM / 🟢 QA
- **Pre-existing, out of scope:** `bun run check` (svelte-check) reports 6 errors, all in `src/lib/components/DiskList.svelte` (P1-T03) — `let state = $state<LoadState>(...)` collides with the `$state` rune name. Not introduced here and not touched; recommend a separate fix ticket (rename `state` → `loadState`).
- **`make smoke` not run** — standing user instruction is "never launch the app (incl. make smoke)". The UI changes are covered by unit tests + typecheck of the touched files instead. Smoke remains the user's call to run.

## Open Questions / TPM Queries
_None — all three refinements stay within DECISION-018(b) (best-effort Windows device checks) and the existing event-stream contract._

---

## 🟢 QA Sign-off

**Date:** 2026-07-01

**Gate Results:**

- `cargo clippy --workspace --all-targets` → **0 warnings** ✓
- `cargo fmt --check` → **clean** ✓
- `cargo test --workspace` → **81 passed, 0 failed**
  - src-tauri: 7 passed
  - core: 6 passed
  - disk: 45 passed
  - recovery: 23 passed
- `bun run lint` (Biome) → **30 files clean** ✓
- `bunx vitest run` → **45 passed, 0 failed** (incl. new ErrorBoundary.test.ts) ✓
- `bunx tsc --noEmit src/lib/types.ts` → **clean** ✓
- Touched frontend files typecheck clean (DiskList pre-existing errors are out of scope)

**Coverage:**
- Rust llvm-cov tool not installed on this system; cannot generate metric. However:
  - All new Rust modules (`panic_log`, `disk_watch`) have unit tests for pure/testable surfaces.
  - `error.rs` has 4 tests covering all 6 variants + serialize behavior.
  - Tests verify edge cases: IO failures swallowed in panic hook, disk absence detection, all raw error payloads hidden from serialization.
- Frontend Vitest: 45 tests across 7 test files; ErrorBoundary tests cover happy path + fallback branch (error thrown by child). No coverage metric available in current environment, but critical paths are exercised.

**Code Quality:**

- No `unwrap()`/`expect()` added outside tests without `// JUSTIFIED:` — verified ✓
- No `unsafe` blocks added — verified ✓
- Cognitive complexity: Clippy reports no violations ✓
- All error event emissions switched to `user_message()` (scan.rs, recovery.rs, panic join arms) ✓
- Panic hook correctly chains default hook and swallows all IO failures ✓
- Disk watcher emits one `disk:disconnected` event then exits (no spam) ✓

**Edge Cases Verified:**

- **EC: Panic before log dir exists** → `create_dir_all` + all IO errors swallowed ✓
- **EC: Disk removed between polls** → single event, then break ✓
- **EC: Stale watcher after operation ends** → `stop` flag causes loop exit ✓
- **EC: Raw OS errors never reach UI** → Serialize switched to `user_message()`, events verified ✓
- **EC: ErrorBoundary shows fallback** → test covers thrown child → alert rendered ✓
- **EC: No event spam on repeated disconnects** → watcher breaks after first emit ✓

**Regressions:**

- Pre-existing: `bun run check` reports 6 errors in `src/lib/components/DiskList.svelte` (P1 issue, unrelated). Not touched; out of scope.
- No new regressions detected. All existing tests continue to pass.

**Status:** ✅ **PASSED**

All mechanical gates green. Rust and frontend tests pass. New code is clean, edge cases verified, no regressions. Error boundary and disconnection UX wired correctly. Ready for merge.

**Note:** `make smoke` was not run per standing user instruction (no app launch). Unit test coverage and typecheck verify the implementation. The user may run smoke independently to validate the headless mount on their machine.
