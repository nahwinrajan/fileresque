# 03-p0-design-system
**Task ID:** P0-T03
**Phase:** P0 — Foundation
**Owner:** 🔵 Developer
**Status:** Done

---

## Overview

Build the core Svelte 5 UI component library for FileResque: Button, Card, Badge, ProgressBar,
Modal, DiskList (placeholder), and FileTable (placeholder). Also create the shared TypeScript
type definitions mirroring Rust core types and the `formatBytes`/`formatDate` utility functions.
All components use Svelte 5 runes, GSAP for interactive state animations (DECISION-007), CSS
custom properties from `tokens.css` only (no hardcoded colour values), and the Firmament dark
palette (DECISION-008).

## Scope

- `src/lib/types.ts` — TypeScript interfaces mirroring `crates/core/src/types.rs`
- `src/lib/utils/format.ts` — `formatBytes` and `formatDate` helpers
- `src/lib/utils/format.test.ts` — table-driven tests for both helpers
- `src/lib/components/Button.svelte` — primary/secondary/ghost/danger, sm/md/lg, GSAP press
- `src/lib/components/Button.test.ts` — 5 test cases covering all branches
- `src/lib/components/Card.svelte` — neumorphic container, selected/hoverable states
- `src/lib/components/Badge.svelte` — compact status labels, 9 variant colours
- `src/lib/components/ProgressBar.svelte` — GSAP-animated fill, scanning shimmer class
- `src/lib/components/Modal.svelte` — overlay with GSAP entrance, focus trap, Escape key
- `src/lib/components/DiskList.svelte` — P1 placeholder with empty state
- `src/lib/components/FileTable.svelte` — P2 placeholder with empty state
- `src/lib/components/index.ts` — barrel export for all components

## Out of Scope

- Full DiskList implementation (P1-T03)
- Full FileTable virtualised rendering (P2-T05)
- Light theme — prohibited by DECISION-008
- Any network calls

## Dependencies

- Blocked by: P0-T01 (scaffold — complete), P0-T02 (CI — complete)
- Requires: `src/lib/styles/tokens.css` (already written, DO NOT modify)
- Requires: `gsap` and `lucide-svelte` in node_modules (already present)
- Requires: `@testing-library/svelte` v5 and `vitest` in devDependencies (already present)

---

## Developer Plan

### Module structure

```
src/lib/
  types.ts                    — DiskInfo, DeletedFileEntry, ProbabilityTier, ProbabilityReport
  utils/
    format.ts                 — formatBytes(bytes, decimals), formatDate(epochSeconds)
    format.test.ts            — table-driven Vitest tests
  components/
    Button.svelte             — interactive button with GSAP shadow physics
    Button.test.ts            — @testing-library/svelte tests
    Card.svelte               — neumorphic container
    Badge.svelte              — status chip
    ProgressBar.svelte        — GSAP width animation
    Modal.svelte              — overlay with GSAP entrance + focus trap
    DiskList.svelte           — disk listing placeholder
    FileTable.svelte          — file table placeholder
    index.ts                  — barrel re-export
```

### Key design decisions

1. **GSAP in tests**: GSAP is mocked via `vi.mock('gsap', ...)` so animation calls do not
   fail in jsdom. Tests assert DOM structure and behavior, not animation values.
2. **`window.matchMedia` guard**: All components check `typeof window !== 'undefined'` before
   calling `matchMedia`. Tests define the mock in `beforeAll`.
3. **`color-mix` for un-tokenised semi-transparent colours**: Modal backdrop
   (`color-mix(in srgb, var(--color-bg-void) 85%, transparent)`) and Badge info variant
   (`color-mix(in srgb, var(--color-accent-info) 12%, transparent)`) use CSS color-mix to
   avoid hardcoding rgba values. WKWebView on macOS 13+ supports color-mix.
4. **Ghost button GSAP skip**: Ghost buttons have `box-shadow: none` by default. The shadow
   GSAP animation is skipped for ghost variant to avoid incorrect shadow injection.
5. **Svelte 5 `<svelte:window>`**: Modal Escape key handler is attached via
   `<svelte:window onkeydown={...}>` inside `{#if open}` so it activates only while open.
6. **Tween cleanup**: ProgressBar kills previous GSAP tween before starting a new one via
   `$effect` cleanup return function.

### Function signatures (utilities)

```typescript
// src/lib/utils/format.ts
export function formatBytes(bytes: number, decimals?: number): string
export function formatDate(epochSeconds: number | null): string
```

### Component prop interfaces — see task spec (mirrored from task prompt)

## Edge Cases

- `formatBytes(0)` → '0 B' (guard against `Math.log(0) = -Infinity`)
- `formatBytes(negative)` → should handle gracefully (treat as 0 or abs value)
- `formatDate(null)` → '—'
- `ProgressBar(value=0)` → 0% width, no GSAP crash
- `ProgressBar(value > max)` → clamped to 100%
- `Button(disabled=true)` + click → handler NOT called
- `Button(loading=true)` + click → handler NOT called, spinner shown
- `Modal(open=false)` → DOM not rendered (Svelte `{#if}`)
- `Modal(open=true → false)` → element unmounted, svelte:window listener removed
- Ghost button → no shadow animation applied
- DiskList(disks=[])` → empty state shown
- DiskList(disks=[...])` → disk cards shown

## Test Plan

### format.test.ts

| Case | Input | Expected | Branch |
|------|-------|----------|--------|
| happy_path_zero | `formatBytes(0)` | `'0 B'` | zero guard |
| happy_path_bytes | `formatBytes(500)` | `'500 B'` | sub-KB |
| happy_path_kb | `formatBytes(1024)` | `'1 KB'` | KB boundary |
| happy_path_mb | `formatBytes(1048576)` | `'1 MB'` | MB boundary |
| happy_path_gb | `formatBytes(1073741824)` | `'1 GB'` | GB boundary |
| happy_path_decimals | `formatBytes(1500, 2)` | `'1.46 KB'` | custom decimals |
| happy_path_date_null | `formatDate(null)` | `'—'` | null guard |
| happy_path_date_valid | `formatDate(0)` | non-empty string | valid epoch |

### Button.test.ts

| Case | Input | Expected | Branch |
|------|-------|----------|--------|
| renders_primary | `variant='primary'` | button has `btn--primary` class | variant render |
| renders_disabled | `disabled=true` | button has `disabled` attribute | disabled state |
| calls_onclick | click event | onclick spy called once | click handler |
| no_onclick_when_disabled | `disabled=true` + click | onclick spy NOT called | disabled guard |
| shows_spinner_when_loading | `loading=true` | `aria-busy=true` | loading state |

---

## Implementation Notes

### Files Produced

| File | Lines | Notes |
|------|-------|-------|
| `src/lib/types.ts` | ~60 | TypeScript mirrors of `crates/core/src/types.rs` — all union types use string literals not enums |
| `src/lib/utils/format.ts` | ~15 | `formatBytes` (0–TB, clamped at TB) + `formatDate` (epoch seconds → locale string, null → em-dash) |
| `src/lib/utils/format.test.ts` | ~90 | 14 table-driven Vitest cases; covers zero guard, negative, all five byte magnitudes, null date, valid date |
| `src/lib/components/Button.svelte` | ~222 | GSAP hover (translate x:-1 y:-1), press (shadow-inset), release (elastic shadow-outset); loading spinner via `LoaderCircle`; ghost variant bypasses shadow animations |
| `src/lib/components/Button.test.ts` | ~80 | 5 tests with GSAP mock + `window.matchMedia` stub; uses guard `if (!btn) throw` instead of `!` assertions |
| `src/lib/components/Card.svelte` | ~115 | Hover GSAP shadow lift via imperative `$effect` + `addEventListener` (avoids `a11y_no_static_element_interactions`) |
| `src/lib/components/Badge.svelte` | ~100 | 9 variants; `info` uses `color-mix()` for 12% opacity background; uppercase letter-spacing |
| `src/lib/components/ProgressBar.svelte` | ~100 | GSAP width tween with kill-before-restart pattern; `scanning` class from tokens.css for shimmer |
| `src/lib/components/Modal.svelte` | ~205 | `<svelte:window>` at root; `panelEl = $state()` for `bind:this` inside `{#if}`; backdrop `contains()` check replaces panel `stopPropagation` |
| `src/lib/components/DiskList.svelte` | ~188 | P1 placeholder; renders disk cards with icon, size, filesystem badge, lock indicator |
| `src/lib/components/FileTable.svelte` | ~70 | P2 placeholder; empty state or count label |
| `src/lib/components/index.ts` | 8 | Barrel export |
| `src/lib/index.ts` | 5 | Re-exports components, types, utils |

### Non-Obvious Decisions Made During Implementation

1. **Modal panel click isolation**: Changed from `onclick={stopPropagation}` on the panel div to a `panelEl.contains(e.target)` check in the backdrop handler. This eliminates the `a11y_click_events_have_key_events` Svelte warning while keeping identical runtime behaviour.

2. **Biome + Svelte false positives**: Biome 1.9.4 does not parse the Svelte template section, so it flags imports used only in `{#if}` blocks as unused. Fixed by adding `biome.json` overrides for `**/*.svelte` disabling `noUnusedImports` and `noUnusedVariables`. A `biome-ignore` comment on `panelEl`'s `let` declaration handles the `useConst` false positive (Biome cannot see `bind:this` reassignment from the template).

3. **`@testing-library/svelte/vite` plugin**: The `svelteTesting()` Vite plugin was added to `vite.config.ts`. Without it, Vitest resolved Svelte to `index-server.js` (the default export), causing `mount(...)` to throw `lifecycle_function_unavailable`. The plugin prepends the `browser` condition to `resolve.conditions` so tests use the client bundle.

4. **`@types/node` install**: `vite.config.ts` uses `process.env.TAURI_DEV_HOST`. This was a pre-existing error in the scaffold — fixed by adding `@types/node` as a dev dependency.

5. **Card.svelte imperative event listeners**: GSAP hover animation on a non-interactive `<div>` was moved from inline Svelte attributes (`onmouseenter`) to `addEventListener` in `$effect`. This avoids the Svelte `a11y_no_static_element_interactions` diagnostic.

### Task Completion Checklist

- [x] `cargo clippy` — N/A (no Rust code in this task)
- [x] `cargo fmt` — N/A
- [x] Cognitive complexity <= 15 — all functions are <10 complexity units
- [x] Unit tests written (table-driven); 19 tests total — `format.test.ts` (14), `Button.test.ts` (5)
- [x] No `unwrap()`/`expect()` without `// JUSTIFIED:` — N/A (TypeScript task)
- [x] All `unsafe` blocks have `// SAFETY:` — N/A (no unsafe)
- [x] `bun biome check src/` — 0 errors, 0 warnings
- [x] `bun vitest run` — 19/19 tests passing
- [x] `bun run check` (svelte-check) — 0 errors, 0 warnings
- [x] Planning doc updated with `## Implementation Notes`

## Open Questions / TPM Queries

None — all design questions resolved via DECISION-007 through DECISION-012.

---

## QA Sign-off

**QA Agent:** 🟢  
**Date:** 2026-06-27  
**Status:** APPROVED

**Checks performed:**
- [x] All 7 components present: Button, Card, Badge, ProgressBar, Modal, DiskList, FileTable
- [x] Component barrel index (src/lib/components/index.ts) exports all 7
- [x] Svelte 5 runes syntax used throughout (`$props()`, `$state()`, `$derived()`, `$effect()` — no `export let`)
- [x] All CSS uses `var(--token)` — no hardcoded hex values in style blocks
- [x] No box-shadow CSS transitions (GSAP only for animations)
- [x] No light-mode media queries (dark palette only, per DECISION-008)
- [x] src/lib/types.ts defines DiskInfo, DeletedFileEntry, ProbabilityReport, ProbabilityTier
- [x] src/lib/utils/format.ts exports formatBytes and formatDate
- [x] vitest: 19/19 tests passing (14 format tests + 5 Button tests)
- [x] bun biome check src/: 0 errors
- [x] bun run check (svelte-check): 0 errors, 0 warnings
- [x] cargo clippy: still 0 warnings (no Rust changes, P0-T01/T02 still passing)

**Test coverage:** Frontend component tests verified; format utility tests all passing.

**Result:** P0-T03 — DONE ✅

## Security Sign-off

[N/A — no unsafe blocks, no disk I/O, no entitlement changes]
