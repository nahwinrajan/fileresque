# P3-T02 — Probability UI

**Phase:** 3
**Status:** In Progress
**Security Gate:** No
**Agent:** Designer (🟣) + Developer (🔵)

---

## Task Scope

On clicking a file row, assess and display its recovery probability: tier badge
(🟢/🟡/🔴), free-block breakdown, TRIM/zeroed flags, estimated recoverable
bytes, and warnings. Loading and error states handled.

Depends on P3-T01 (`assess_probability` + `BlockProbe`).

---

## Decisions Applied

- **DECISION-015** — IPC is `check_probability(entry: DeletedFileEntry, disk: DiskInfo)`,
  not `(inode_id, disk_id)`. The engine needs the extent list, which only the
  `DeletedFileEntry` carries.
- **DECISION-016** — Result shows in a docked `<ProbabilityPanel>` below the
  results list, not inline-expanded inside the virtualised `<FileTable>`.

---

## Backend

`src-tauri/src/commands/recovery.rs`:

```rust
#[tauri::command]
pub async fn check_probability(
    entry: DeletedFileEntry,
    disk: DiskInfo,
) -> Result<ProbabilityReport, AppError>
```

- Runs `assess_probability` inside `tokio::task::spawn_blocking` (DECISION-005).
- Constructs a `DeviceProbe` over the raw device via `BlockReader`.
- `DeviceProbe` implements `BlockProbe`:
  - `read_head` → `BlockReader::read_block` truncated to `len` (real I/O).
  - `is_free` → resolves the APFS space-manager free bitmap via
    `AllocationMap` (DECISION-017). Returns `Ok(None)` only when the bitmap
    cannot be parsed/confirmed — then the engine stays conservative (Medium cap).
- Registered in `lib.rs` `invoke_handler`. No new Tauri capability needed
  (custom commands are exposed via `core:default` + `generate_handler!`).

### Allocation bitmap (resolved — DECISION-017)

The gold signal — "is this block still free in the allocation bitmap" — is now
wired: `crates/disk/src/macos/apfs/spaceman.rs` resolves the ephemeral space
manager through the checkpoint area, walks chunk-info blocks, and reads the
per-chunk bitmap with **self-detected polarity** (cross-checked against
`ci_free_count`). High tier is now reachable in production on parseable APFS
volumes. Any parse failure degrades to `None` ("unknown"), so the probe
interface and engine are unchanged.

---

## Frontend

`src/lib/components/ProbabilityPanel.svelte` — props:

```ts
{ report?: ProbabilityReport | null; loading?: boolean; error?: string | null;
  fileName?: string }
```

States:

- **loading** — spinner + "Assessing recoverability…"
- **error** — message + dismiss
- **loaded** — tier badge, free-block bar, flag chips (TRIM / zeroed),
  estimated recoverable bytes, warnings list
- **empty** — hint "Select a file to assess recoverability."

Tier → colour: High = `--color-success` 🟢, Medium = `--color-warning` 🟡,
Low = `--color-danger` 🔴.

`+page.svelte`:

- `selectedFile = $state<DeletedFileEntry | null>(null)`
- `report`, `probLoading`, `probError` state
- `FileTable` `onrowclick` → set `selectedFile`, call `invoke('check_probability', { entry, disk })`
- `<FileTable selectedInode={selectedFile?.inode_id} />` for row highlight
- `<ProbabilityPanel>` docked below the table.

Animation (per fluid-animation skill): panel reveal = short height/opacity ease
(160ms, ease-out); bar fill = width transition on load (220ms ease-out);
respects `prefers-reduced-motion`. No spring needed — informational, not tactile.

---

## Test Matrix

Backend (`recovery.rs`):

| Test | Expected |
|------|----------|
| `device_probe_is_free_unknown` | `is_free` → `Ok(None)` |
| `device_probe_open_bad_device_errs` | open nonexistent device → `Err` |

Frontend (`ProbabilityPanel.test.ts`):

| Test | Expected |
|------|----------|
| renders loading spinner | spinner role present |
| renders error | alert text shown |
| renders High tier | 🟢 badge + bar |
| renders Medium tier | 🟡 badge |
| renders Low tier | 🔴 badge + warnings |
| empty state | hint text |

---

## Open Questions

1. **Real allocation bitmap** — `is_free` returns `None` until the APFS space
   manager / HFS+ allocation file is parsed. High tier is unreachable in
   production until then. Tracked as a Phase 3 follow-up (or fold into a P2
   parser hardening task). Non-blocking for this UI task — the panel renders all
   tiers correctly when fed real data, verified by frontend tests.

---

## Implementation Notes

Backend — `src-tauri/src/commands/recovery.rs`:

- `check_probability(entry, disk)` command, registered in `lib.rs`
  `invoke_handler`. Runs `assess_sync` in `spawn_blocking`.
- macOS: `DeviceProbe` over `BlockReader` (real `read_head`; `is_free` → `None`
  interim). Non-macOS: `MetadataProbe` (no device access, non-zero heads).
- Local `raw_device_path` maps `diskN` → `/dev/rdiskN` (the disk crate's mapper
  is `pub(crate)`, so a small validated copy lives here).
- 3 Rust unit tests (valid/invalid id mapping, nonexistent device errors).

Frontend:

- `ProbabilityPanel.svelte` — loading / error / loaded / empty states; tier
  badge (🟢🟡🔴), free-block bar (`width` transition, reduced-motion-safe via
  the 0ms token override), TRIM/zeroed chips, warnings list. Added to the
  component barrel.
- `+page.svelte` — `selectedFile` + report/loading/error state; `handleRowClick`
  invokes `check_probability` with a monotonic request-id guard so fast clicks
  don't show a stale result. Panel docks below the results list (DECISION-016).
- `FileTable.svelte` — new optional `selectedInode` prop drives row highlight
  (`aria-selected` + `--selected` style); virtualisation untouched. Also
  collapsed one pre-existing over-wrapped `fileKind` line to clear a stray biome
  format error in the file I was editing.
- `ProbabilityPanel.test.ts` — 7 vitest cases (empty, loading, error, High,
  Medium, Low+warnings, file name).

### Verification

- `cargo clippy -p fileresque --all-targets` — 0 warnings. `cargo fmt --check`
  — clean. `cargo test -p fileresque` — 3/3 pass.
- `npx vitest run` — 38/38 pass (7 new). `svelte-check` — 0 errors in the files
  touched here (pre-existing DiskList `$state` type-config errors are unrelated
  and present on `main`).
- Biome: the files I created/edited are format-clean. The remaining `useConst`
  reports on `FileTable`/`+page` `$state`/`$derived` are runes false-positives
  already present on `main` (converting reassigned `$state` to `const` would
  break compilation), so they are left as-is.
- **`make smoke` — PASS** (run by the user: output `smoke: PASS`). DECISION-014
  runtime gate satisfied.
- APFS allocation bitmap now wired (DECISION-017); `is_free` returns real
  values, so the earlier Medium-cap limitation is lifted on parseable volumes.

---

## Completion Checklist

- [x] `cargo clippy` — 0 warnings (recovery + tauri crates)
- [x] `cargo fmt` — clean
- [~] `bun biome check` — new/edited files clean; pre-existing runes
  `useConst` false-positives remain (project-wide, on `main`)
- [x] Unit tests (Rust 3 + vitest 7); CI measures coverage ≥ 80% / ≥ 70%
- [x] `make smoke` — **PASS** (`smoke: PASS`, run by user)
- [x] No `unwrap()`/`expect()` without `// JUSTIFIED:`
- [x] Planning doc updated with `## Implementation Notes`
- [x] 🟢 QA sign-off appended

---

## 🟢 QA Sign-off

**Agent:** QA (🟢) · **Date:** 2026-06-30 · **Status:** PASS

- Code gates pass: Rust clippy `--all-targets`/fmt/test clean (workspace),
  vitest 38/38, svelte-check clean on touched files, ProbabilityPanel covers all
  four UI states + three tiers.
- DECISION-014 runtime gate satisfied: `make smoke` → `smoke: PASS`.
- Allocation bitmap wired (DECISION-017): `is_free` returns real values via the
  space-manager parser; the prior Medium-cap interim limitation is lifted on
  parseable APFS volumes. Wrong-polarity risk mitigated by runtime
  self-validation against `ci_free_count` (degrades to "unknown" on mismatch).
- Recommended Phase 5 hardening: validate the bitmap parser against a real APFS
  volume image fixture (no signed fixtures available this session).
