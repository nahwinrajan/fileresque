# P4-T03 — Recovery Progress UI

**Task ID:** P4-T03
**Phase:** 4 — File Recovery
**Owner:** Designer (🟣) + Developer (🔵)
**Status:** QA Review (smoke pending — see note)
**Security Gate:** No

---

## Overview

The user-facing recovery flow: confirm what/where, watch live progress, see the
result. A single `<RecoveryModal>` drives the whole lifecycle off the P4-T01/T02
commands and the `recovery:*` events.

## Scope

- `<RecoveryModal>` — confirm → recovering → done, plus pre-flight `blocked` state
- Pre-flight surfaced before any write (calls `preflight_recovery`)
- Live per-file progress (`recovery:progress` → `<ProgressBar>`)
- Completion view: success / partial (bad sectors) / failed / cancelled
- Cancel button (`cancel_recovery`); "Open folder" on completion (`shell.open`)
- "Recover this file…" action in `+page.svelte` (native folder pick → modal)
- TS event/result types in `src/lib/types.ts`

## Out of Scope

- **Multi-file selection.** This task recovers the single selected file (the one
  shown in `<ProbabilityPanel>`). Batch selection is a `<FileTable>` change
  (same reasoning as DECISION-016, which kept the virtual list untouched) and is
  future scope — the backend `recover_files` already accepts a `Vec`, so the UI
  can grow into it without backend change.
- Reveal-single-file (needs an extra shell permission); we open the folder.

## Dependencies

- Blocked by: P0-T03 (design system / Modal / ProgressBar / Button), P4-T02

---

## [DES/DEV] Plan

`<RecoveryModal>` composes the existing `<Modal>` (focus trap, Escape, GSAP
entrance, reduced-motion guard) with a `phase` state machine:

```
preflight ─▶ blocked        (pre-flight failed → show errors, no Recover)
          └▶ confirming ─▶ recovering ─▶ done (success | partial | failed | cancelled)
```

- On open with a `destPath`, `$effect` runs `preflight_recovery([file], disk, dest)`.
- `confirming` → "Recover" → `recover_files`, attach `recovery:*` listeners,
  `recovering`. Backdrop/Escape close is suppressed while `recovering`; only the
  explicit "Cancel recovery" button stops it (`cancel_recovery`).
- `recovery:progress` feeds `<ProgressBar value=bytes_written max=size_bytes>`.
- `recovery:complete` → `done`; the view branches on `cancelled` / `failed` /
  `partial` / success and offers "Open folder".

Parent (`+page.svelte`) owns the native folder pick (`pick_destination_folder`,
`Ok(None)` = cancelled → no-op) and toggles the modal.

## Edge Cases

- Pre-flight fails → `blocked`, lists each error message, Recover hidden
- Pre-flight IPC throws → `blocked` with the error text
- `recover_files` throws synchronously → `done` with the error
- Cancel mid-recovery → summary reports `cancelled`, "no partial file left behind"
- Bad sectors → `partial` view names the zero-filled block count
- Closed modal renders nothing (no stray listeners; `$effect` teardown detaches)

## Test Plan

| Case | Input | Expected | Branch |
|------|-------|----------|--------|
| summary | open, preflight ok | file name + destination shown | meta |
| confirming | preflight ok | "Recover" button present | confirming |
| blocked | preflight !ok (SameDisk + space) | both messages, no Recover | blocked |
| preflight_throws | invoke rejects | error text shown, blocked | error |
| closed | open=false | renders nothing | guard |

`RecoveryModal.test.ts` — 5 tests (Tauri `invoke`/`listen`, GSAP, `matchMedia` mocked).

---

## Implementation Notes

- Self-contained lifecycle component: it owns the preflight + recover invokes and
  the `recovery:*` listeners, mirroring how `<PermissionGate>` owns its check.
  `+page.svelte` only supplies `file`/`disk`/`destPath` and toggles `open`.
- Reuses `<Modal>`/`<ProgressBar>`/`<Button>` — no new primitives, no new GSAP
  usage (motion stays inside `<Modal>`), consistent with the Firmament tokens.
- "Open folder" uses `shell.open(destPath)` (covered by the existing
  `shell:allow-open` capability) — revealing a single file would need a new
  permission, deferred.
- Verified: `bun biome ci .` clean; `bun vitest run` green (43 total, 5 new);
  new component is `svelte-check`-clean (the 6 remaining `svelte-check` errors
  are pre-existing in `DiskList.svelte` and are not in the CI/`make lint` path).

## Task Completion Checklist

- [x] `cargo clippy` / `cargo fmt` — N/A (frontend task); workspace still clean
- [x] Unit tests written (component, Tauri-mocked); 5 green
- [x] `bun biome ci .` clean
- [ ] **`make smoke` — PENDING.** Owner's environment bars the assistant from
  launching the app (incl. `make smoke`). Must be run by the user; this gate
  stays open until the screenshot + zero-console-error artifact is captured
  (DECISION-014). Green unit tests do **not** substitute.
- [x] No `unwrap()`/`expect()` in non-test code
- [x] Planning doc updated with `## Implementation Notes`
- [ ] 🟢 QA sign-off — blocked on `make smoke` above
- [ ] 🔴 Security — N/A

## Open Questions / TPM Queries

None. Single-file vs multi-file recovery resolved as single-file for this task
(see Out of Scope); backend already supports batch for a later UI iteration.
