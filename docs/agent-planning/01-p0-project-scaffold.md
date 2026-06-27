# 01-p0-project-scaffold
**Task ID:** P0-T01
**Phase:** P0 — Foundation
**Owner:** [DEV]
**Status:** QA Review

---

## Overview

Bootstrap the entire project structure: Cargo workspace with three library crates
(`core`, `disk`, `recovery`), the `src-tauri` Tauri 2 binary crate, and the
SvelteKit + TypeScript frontend. After this task the repo compiles cleanly with
`cargo check`, passes `cargo clippy -- -D warnings`, passes `cargo fmt --check`,
and `pnpm install` succeeds. No business logic is implemented yet — all
platform-specific modules are stubs returning empty results.

## Scope

- Root `Cargo.toml` workspace definition (members + shared deps via `[workspace.dependencies]`)
- `.cargo/config.toml` — global `rustflags = ["-D", "warnings"]`
- `clippy.toml` — `cognitive-complexity-threshold = 15`
- `rustfmt.toml` — `edition = "2021"`, `max_width = 100`
- `crates/core` — canonical types (`DiskInfo`, `DeletedFileEntry`, `ProbabilityReport`,
  `PreflightResult`, enums) + `AppError` with `thiserror`
- `crates/disk` — platform-gated stubs (`#[cfg(target_os = "macos")]` / `windows`)
- `crates/recovery` — stub `probability.rs` and `engine.rs`
- `src-tauri` — Tauri 2 binary; single placeholder `greet` command; `tauri.conf.json`
  with hybrid titlebar config (DECISION-010)
- Frontend scaffold: `package.json`, `vite.config.ts`, `svelte.config.js`, `tsconfig.json`,
  `src/app.html`, `src/app.css` (imports existing `tokens.css`), SvelteKit layout +
  placeholder page
- `.gitignore`, `.editorconfig`, `Makefile`

## Out of Scope

- Any actual disk enumeration, FS parsing, or recovery logic (P1+)
- Font files bundling (P0-T03 / DECISION-011)
- Full Disk Access entitlement (P0-T04)
- CI pipeline (P0-T02)
- Light theme (removed per DECISION-008)
- Existing `src/lib/styles/tokens.css` — DO NOT TOUCH (Designer-owned)

## Dependencies

- Blocked by: none (first task)
- DECISION-010 (hybrid titlebar) — `hiddenTitle: true`, `titleBarStyle: "Overlay"`
- DECISION-011 (font bundling) — defer to P0-T03
- DECISION-007 (GSAP) — include in `package.json` dependencies

---

## Developer Plan

### Module Structure

```
Cargo workspace root
├── crates/core/src/
│   ├── lib.rs          — mod declarations
│   ├── types.rs        — all canonical types (canonical to CONTEXT.md)
│   └── error.rs        — AppError (thiserror) + manual Serialize impl
├── crates/disk/src/
│   ├── lib.rs          — cfg-gated mod declarations
│   ├── macos/mod.rs    — re-exports enumerate
│   ├── macos/enumerate.rs — async list_disks() stub
│   ├── windows/mod.rs  — re-exports enumerate
│   └── windows/enumerate.rs — async list_disks() stub
├── crates/recovery/src/
│   ├── lib.rs          — mod declarations
│   ├── probability.rs  — assess_probability() stub
│   └── engine.rs       — recover_file() stub
└── src-tauri/src/
    ├── main.rs         — windows_subsystem cfg attr + main()
    ├── lib.rs          — Tauri builder setup
    └── commands/mod.rs — greet() placeholder
```

### Key Design Decisions Encoded

- `SystemTime` serde: custom inline module `system_time_serde` serialises
  `Option<SystemTime>` as `Option<u64>` (seconds since UNIX epoch). No extra
  crates needed. Precision sufficient for file-deletion timestamps.
- `FileSystem` and `DriveType` enum variants use industry-standard uppercase
  names (`APFS`, `NTFS`, `SSD`, etc.) — `#[allow(clippy::upper_case_acronyms)]`
  applied at enum level.
- `src-tauri/src/commands/mod.rs` uses `#![allow(clippy::must_use_candidate)]`
  because Tauri IPC always consumes the return value.
- `run()` in `lib.rs` documents the `expect()` panic in `# Panics` per
  `clippy::missing_panics_doc`.

### Function Signatures

```rust
// crates/disk/src/macos/enumerate.rs
pub async fn list_disks() -> Result<Vec<DiskInfo>, AppError>

// crates/disk/src/windows/enumerate.rs
pub async fn list_disks() -> Result<Vec<DiskInfo>, AppError>

// crates/recovery/src/probability.rs
pub async fn assess_probability(
    _entry: &DeletedFileEntry,
    _disk: &DiskInfo,
) -> Result<ProbabilityReport, AppError>

// crates/recovery/src/engine.rs
pub async fn recover_file(
    _entry: &DeletedFileEntry,
    _source_disk_id: &str,
    _dest_path: &std::path::Path,
) -> Result<String, AppError>

// src-tauri/src/commands/mod.rs
pub fn greet(name: &str) -> String
```

## Edge Cases

- `SystemTime` before UNIX epoch: `duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO)` —
  serialises as 0 seconds, acceptable for scaffold.
- macOS/Windows platform gating: on macOS only `macos` module compiles; on Windows
  only `windows` module. `cargo clippy` on macOS will only validate macOS paths.
- Empty `src-tauri/icons/` directory: `cargo check` does not validate icon existence
  (bundler step only).

## Test Plan

| Case | Input | Expected | Branch covered |
|------|-------|----------|----------------|
| `disk_info_serializes` | Valid `DiskInfo` with all fields | JSON contains `"disk0"` | Happy path serde round-trip |

(Minimal for scaffold; comprehensive tests in P1 per-feature tasks)

---

## Implementation Notes

### What was built

All files listed in Scope were created. Summary of deviations and notable decisions:

**Rust workspace (`Cargo.toml`, `.cargo/config.toml`, `clippy.toml`, `rustfmt.toml`)**
- Workspace resolver = "2" with shared deps via `[workspace.dependencies]`.
- `-D warnings` enforced globally via `.cargo/config.toml` so all crates inherit it
  without needing `deny(warnings)` in each crate root.

**`crates/core/src/types.rs`**
- `SystemTime` serde implemented via an inline `mod system_time_serde` (no extra crate).
  Serialises as `Option<u64>` seconds-since-epoch; pre-epoch times clamped to 0.
- `clippy::upper_case_acronyms` suppressed at enum level with comment explaining
  the industry-standard naming convention.
- `clippy::ref_option` suppressed on `system_time_serde::serialize` with comment
  explaining that the `&Option<T>` signature is mandatory for serde's `with` attribute.
- Two table-driven tests added: `disk_info_serializes` and
  `deleted_file_entry_round_trips_system_time`.

**`crates/disk` / `crates/recovery` stubs**
- All stub `async fn` stubs carry `#[allow(clippy::unused_async)]` with a comment
  stating the `async` signature is the final API contract; the allow is removed when
  the real implementation is written in P1/P3/P4.
- `IOKit` wrapped in backticks in doc comments to satisfy `clippy::doc_markdown`.

**`src-tauri`**
- `run()` function carries a `# Panics` doc section to satisfy `clippy::missing_panics_doc`.
- `commands/mod.rs` uses `#![allow(clippy::must_use_candidate)]` because Tauri IPC
  always consumes command return values.
- Placeholder icon PNGs (32x32, 128x128, 256x256, RGBA) generated by Python script;
  required because `tauri::generate_context!()` validates icons at proc-macro expansion
  time, not just during bundling. ICO and ICNS placeholders also created.

**Frontend scaffold**
- `src/app.css` imports existing `tokens.css` (not touched).
- `+page.svelte` uses `data-tauri-drag-region` on titlebar div per DECISION-010.
- `+layout.svelte` checks `prefers-reduced-motion` and calls `gsap.globalTimeline.timeScale(0)`.
- `+layout.ts` exports `prerender = true; ssr = false` for SvelteKit static adapter.

### Completion checklist results

- [x] `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings
- [x] `cargo fmt --all -- --check` — clean
- [x] Cognitive complexity ≤ 15 on all new functions (all stubs; no complex logic)
- [x] Unit tests written (table-driven); 2 tests in `fileresque_core` pass
- [x] No `unwrap()`/`expect()` without `// JUSTIFIED:` (only in tests with justification)
- [x] No `unsafe` blocks in this scaffold
- [x] `cargo test --workspace` — 2 passed, 0 failed
- [ ] `pnpm install` — pending (frontend install requires network; run manually)
- [ ] QA sign-off — pending
- [ ] Security sign-off — not required for this task

## Open Questions / TPM Queries

None — all decisions are resolved in `decisions.md`.

---

## QA Sign-off

Pending — [QA] to complete after implementation.

## Security Sign-off

Not required for this task (no `unsafe` blocks, no entitlements, no OS APIs).
