# 02-p0-ci-pipeline
**Task ID:** P0-T02
**Phase:** P0 — Foundation
**Owner:** [DEV] 🔵
**Status:** Done

---

## Overview

Create the GitHub Actions CI pipeline for FileResque. The workflow runs on every push to any branch and on pull requests targeting `main`. It enforces the quality gates mandated by CONTEXT.md: lint (clippy + rustfmt + biome), unit tests, coverage thresholds (≥80% Rust lines, ≥70% frontend), and a compilation smoke-test on both macOS and Windows. No full Tauri bundle is attempted in CI because that requires code-signing secrets.

## Scope

- `.github/workflows/ci.yml` with five jobs: `lint`, `test`, `coverage`, `build-mac`, `build-win`
- `Swatinem/rust-cache@v2` for Cargo caching on all Rust jobs
- `oven-sh/setup-bun@v2` for Bun on all JS jobs
- `cargo llvm-cov` via `taiki-e/install-action` for Rust coverage with `--fail-under-lines 80`
- `codecov/codecov-action@v5` for uploading LCOV report (non-fatal on error)
- Frontend vitest coverage (threshold enforced in `vite.config.ts` at 70%)

## Out of Scope

- Full `bun tauri build` / notarisation — that is P5-T01 (release pipeline)
- Matrix testing across multiple Rust toolchain versions
- Code-signing secrets setup
- Deployment or artefact publishing

## Dependencies

- Blocked by: P0-T01 (project scaffold) — must be complete so `Cargo.toml` workspace, `package.json`, `bun.lock`, and `biome.json` all exist

---

## Developer Plan

### Module structure

```
.github/
└── workflows/
    └── ci.yml
```

Single file; no composite actions required for this scope.

### Job dependency graph

```
lint ──────┐
           ├──► build-mac
test ──────┤
           ├──► build-win
           └──► coverage
```

`lint` and `test` are independent and run in parallel. `coverage` waits on `test` (reuses the same artefact base without re-running from scratch). `build-mac` and `build-win` both wait on `lint` AND `test` to avoid wasting macOS/Windows runner minutes on a broken build.

### Key decisions reflected in the YAML

| Concern | Choice | Reason |
|---------|--------|--------|
| Package manager | `bun install --frozen-lockfile` | DECISION-012: Bun replaces pnpm |
| JS linter/formatter | `bun biome ci .` | DECISION-012: Biome replaces ESLint/Prettier |
| Rust coverage tool | `cargo llvm-cov` via `taiki-e/install-action` | Matches `make coverage` target |
| Coverage threshold | `--fail-under-lines 80` | CONTEXT.md absolute rule |
| Tauri bundle in CI | NOT run | Requires signing certs; only `cargo build --workspace` |
| `svelte-check` / `bun run check` | `continue-on-error: true` | `.svelte-kit` dir may not exist without a `tauri dev` run; warning only |
| Codecov upload | `fail_ci_if_error: false` | Upload failure must not block CI |

### Function signatures / CI step naming convention

All `name:` fields follow sentence-case: `cargo clippy`, `cargo fmt check`, `bun install`, `biome ci`, `cargo test`, `vitest`, `cargo coverage`, `vitest coverage`, `cargo build (macOS)`, `cargo build (Windows)`.

## Edge Cases

- `bun.lock` absent on Windows runner: `--frozen-lockfile` will fail, surfacing the missing lockfile early rather than silently regenerating it.
- `.svelte-kit` directory absent: `bun run check` calls `svelte-kit sync` which writes to `.svelte-kit/`; on a clean checkout this may emit warnings but should not fail; `continue-on-error: true` guards against any edge-case failure.
- `cargo llvm-cov` coverage below 80%: CI fails on the `coverage` job but does not block the `build-*` jobs (they depend only on `lint` and `test`). This matches the intent that build verification is independent of coverage gate.

## Test Plan

CI pipelines are validated by their YAML syntax and by observing the first green run. The table below describes what each job verifies:

| Case | Job | Input | Expected outcome |
|------|-----|-------|-----------------|
| happy_path_lint | `lint` | Clean workspace | clippy 0 warnings, fmt clean, biome clean |
| happy_path_test | `test` | Passing tests | all workspace tests pass, vitest passes |
| happy_path_coverage | `coverage` | ≥80% Rust coverage | llvm-cov exits 0, lcov.info uploaded |
| happy_path_build_mac | `build-mac` | macOS runner | `cargo build --workspace` exits 0 |
| happy_path_build_win | `build-win` | Windows runner | `cargo build --workspace` exits 0 |
| branch_lint_fail | `lint` | clippy warning | job fails, build-mac/win never start |
| branch_test_fail | `test` | failing test | job fails, coverage + build-mac/win never start |
| branch_coverage_low | `coverage` | <80% line coverage | coverage job fails; does not block build jobs |
| branch_lockfile_drift | `test` | missing/stale bun.lock | `bun install --frozen-lockfile` fails immediately |

---

## Implementation Notes

### What was built

Created `.github/workflows/ci.yml` with five jobs matching the plan above:

- **`lint`** (`ubuntu-latest`): rust-toolchain stable with clippy+rustfmt components, Swatinem cache, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, setup-bun, `bun install --frozen-lockfile`, `bun biome ci .`
- **`test`** (`ubuntu-latest`): rust-toolchain stable, Swatinem cache, `cargo test --workspace`, setup-bun, `bun install --frozen-lockfile`, `bun vitest run`
- **`coverage`** (`ubuntu-latest`, needs: test): rust-toolchain stable, Swatinem cache, `taiki-e/install-action@cargo-llvm-cov`, `cargo llvm-cov --workspace --lcov --output-path lcov.info --fail-under-lines 80`, setup-bun, `bun install --frozen-lockfile`, `bun vitest run --coverage`, `codecov/codecov-action@v5` with `fail_ci_if_error: false`
- **`build-mac`** (`macos-latest`, needs: [lint, test]): rust-toolchain stable, Swatinem cache, setup-bun, `bun install --frozen-lockfile`, `bun run check` (continue-on-error), `cargo build --workspace`
- **`build-win`** (`windows-latest`, needs: [lint, test]): identical structure to build-mac

### Deviations from plan

None. Implementation matches plan exactly.

### Files created

- `.github/workflows/ci.yml`

---

## Open Questions / TPM Queries

None.

---

## QA Sign-off

**QA Agent:** 🟢  
**Date:** 2026-06-26  
**Status:** APPROVED

**Checks performed:**
- [x] Planning doc complete with all required sections (Overview, Scope, Plan, Implementation Notes)
- [x] ci.yml YAML syntax valid (Python yaml parser)
- [x] All 5 jobs present with correct runners (lint/test/coverage ubuntu-latest, build-mac macOS, build-win windows)
- [x] Job dependency graph correct (coverage→test, build-mac/win→[lint,test])
- [x] Bun used throughout (28 refs, 0 pnpm refs)
- [x] Biome used for JS lint (2 refs, 0 eslint/prettier refs)
- [x] Rust coverage gate: --fail-under-lines 80 present in coverage job
- [x] Frontend coverage threshold: 70% for lines/functions/branches/statements in vite.config.ts
- [x] Swatinem/rust-cache@v2 present in 5 jobs (lint, test, coverage, build-mac, build-win)
- [x] oven-sh/setup-bun@v2 present in 5 jobs
- [x] --frozen-lockfile used in all 5 bun install steps
- [x] CARGO_TERM_COLOR: always set in env
- [x] codecov/codecov-action@v5 configured with fail_ci_if_error: false
- [x] svelte-kit sync (bun run check) marked continue-on-error: true on both macOS/Windows builds
- [x] Triggers: push to all branches, pull_request to main only

**Result:** P0-T02 — CI Pipeline — DONE ✅

## Security Sign-off

[N/A — no security gate for CI configuration]
