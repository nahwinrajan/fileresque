# Planning Doc — P2-T05 Scan Results UI

**Task ID:** P2-T05  
**Phase:** P2 — File System Scanning  
**Status:** Implemented  
**Agent:** 🔵 Developer + 🟣 Designer  
**Security Gate:** No

---

## Background

P2-T05 upgrades the frontend to a full two-panel scan application:
- Left panel: disk selection + scan/cancel button
- Right panel: live progress banner + virtualized file table

---

## Design

### App state machine

```
idle → scanning → complete
                → error → (retry → scanning)
```

### Virtual list (FileTable.svelte)

Renders 10 000+ rows without layout jank via CSS transform–based windowing:

1. Outer container has `overflow-y: auto` and reports `clientHeight` via `ResizeObserver`
2. Inner spacer div has `height = files.length × ROW_HEIGHT` to maintain correct scrollbar
3. Rendered rows div uses `transform: translateY(offsetTop)` — GPU-composited, no layout
4. Only rows in `[visibleStart, visibleEnd]` (viewport ± `BUFFER_ROWS = 15`) are in the DOM

`ROW_HEIGHT = 40px` — fixed for simplicity; all rows are equal height.

### Tauri event listeners

```ts
listen('scan:file_found', ({ payload }) => { files = [...files, payload]; })
listen('scan:progress',   ({ payload }) => { filesFound = payload.files_found; })
listen('scan:complete',   ({ payload }) => { appState = 'complete'; })
listen('scan:error',      ({ payload }) => { appState = 'error'; })
```

Listeners are attached on scan start and removed on scan end / component destroy
to avoid duplicate events.

### Indeterminate progress bar

A CSS-only animation (`scan-indeterminate` keyframe) on a `div` with
`overflow: hidden`. No dependency on the `ProgressBar` component (which only
supports determinate progress).

---

## Files Changed

- `src/lib/components/FileTable.svelte` — full rewrite with virtual list
- `src/routes/+page.svelte` — full scan flow (disk selection, progress, results)

---

## Completion Checklist

- [x] Virtual list renders 10 000 rows without layout jank (CSS transform)
- [x] Live progress banner during scan
- [x] Cancel button wired to `cancel_scan` command
- [x] Scan summary on completion
- [x] Error state with retry button
- [x] File type icons (image/video/audio/doc/unknown)
- [ ] Sort by name / size / date — deferred post P2
- [ ] Filter by type / date — deferred post P2
- [ ] 🟢 QA sign-off

---

## Implementation Notes — Post-Implementation Defects (2026-06-27)

This task was marked **Implemented** with green `cargo test` + `vitest`, but the
app would not render: `make dev` showed a blank window. Two independent defects,
neither caught by unit tests:

1. **Dual Svelte runtime instance** — `@sveltejs/vite-plugin-svelte@4.0.4` was
   installed against `vite@6.4.3` (plugin peer range is `vite ^5`). The major
   mismatch produced two Svelte runtime copies; `import { onDestroy }` read a
   null component context → `Cannot read properties of null (reading 'r')` on
   mount → blank page. Reproduced in dev AND the production rollup build. Fixed
   by upgrading to `@sveltejs/vite-plugin-svelte@5.1.1`. See **DECISION-013**.
2. **Stub `icon.icns`** — Phase 0 left an 8-byte placeholder icon; Tauri dev
   codegen embeds it and `NSImage::initWithData` panics on invalid data, killing
   startup before the window loaded. Fixed by regenerating a valid ICNS.

Supporting frontend bugs fixed at the same time (recorded in git, not here):
`+layout.svelte` used Svelte 4 `<slot />` (→ `{@render children()}`); `--color-border`
token was undefined; `static/favicon.png` was missing (broke prerender).

**Why unit tests missed all of this:** `vitest` runs in jsdom with the
`svelteTesting` browser condition, which resolves Svelte differently than the
real Vite/Tauri build — so the exact failing code path was never executed. The
binary itself was never launched during review.

**Process gap & fix:** the QA gate passed on unit tests alone. A mandatory,
mechanical runtime smoke check (`make smoke`) with a captured artifact is now
required before any QA sign-off. See **DECISION-014**. This task's QA gate stays
**open** until it passes `make smoke`.
