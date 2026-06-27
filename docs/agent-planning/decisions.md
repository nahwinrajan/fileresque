# Decision Log

> Append-only. Managed by 🟠 TPM. All architectural and product decisions are recorded here with rationale.

---

## [DECISION-001] — Framework Selection: Tauri 2 + Rust over Golang WASM

**Date:** 2026-06-26  
**Decided by:** TPM  
**Context:** Initial technology selection for file recovery desktop application.  
**Options considered:**
- A: Golang WASM — familiar Go ecosystem, but WASM sandbox blocks raw disk I/O and privilege escalation
- B: Tauri 2 + Rust — native Rust backend with full OS access, WebView frontend, code-signed binaries
**Decision:** B — Tauri 2 + Rust  
**Rationale:** Raw disk access at `/dev/rdiskN` and `\\.\PhysicalDriveN` is fundamentally incompatible with a WASM sandbox. Rust provides compile-time memory safety which is critical when parsing untrusted binary data from disk. Tauri 2 supports custom macOS entitlements and Windows UAC manifests required for elevated disk access.  
**Consequences:** Rust learning curve for frontend developers; however, business logic is cleanly separated into crates, so frontend work remains in TypeScript/Svelte.

---

## [DECISION-002] — macOS as Primary Target

**Date:** 2026-06-26  
**Decided by:** TPM  
**Context:** Resource allocation across two platforms.  
**Options considered:**
- A: Equal priority macOS + Windows from the start
- B: macOS primary; Windows secondary (same codebase, platform-gated modules)
**Decision:** B — macOS primary  
**Rationale:** Owner-specified priority. APFS parsing is more complex than NTFS; tackling the harder problem first reduces risk. The shared `crates/core` types and trait abstractions ensure Windows can be added without architectural rework.  
**Consequences:** NTFS parser (P2-T03) and Windows disk enum (P1-T02) are Phase 2 tasks but share the same crate structure as macOS equivalents. Windows installer (P5-T02) is final phase.

---

## [DECISION-003] — No App Store Distribution for v1

**Date:** 2026-06-26  
**Decided by:** TPM  
**Context:** macOS App Store requires entitlement approval for raw disk access; review timeline is uncertain.  
**Options considered:**
- A: Target Mac App Store for wider distribution
- B: Direct download (DMG + notarisation) for v1; App Store as v2 target
**Decision:** B — Direct download for v1  
**Rationale:** The `com.apple.security.files.all` entitlement (Full Disk Access) is unlikely to be approved by Apple for a third-party app without a lengthy review process. Notarised DMG allows Gatekeeper-compatible distribution on Day 1.  
**Consequences:** P5-T01 focuses on notarisation only; App Store provisioning is out of scope for v1.

---

## [DECISION-004] — Frontend Framework: Svelte

**Date:** 2026-06-26  
**Decided by:** TPM  
**Context:** Tauri 2 supports React, Svelte, Vue, and vanilla JS. Need lightweight reactive UI.  
**Options considered:**
- A: React — large ecosystem, familiar to most frontend devs
- B: Svelte — compile-time reactivity, smaller bundle, no virtual DOM overhead
- C: Vue — good middle ground
**Decision:** B — Svelte  
**Rationale:** Tauri's WebView is a constrained environment; Svelte's compile-time approach produces smaller, faster JS bundles than React. File table rendering (10k+ rows) benefits from Svelte's direct DOM manipulation. Aligns with minimum-dependencies philosophy.  
**Consequences:** Developers unfamiliar with Svelte must review Svelte 5 runes syntax; however Svelte's learning curve is lower than React's ecosystem complexity.

---

## [DECISION-006] — App Name: FileResque

**Date:** 2026-06-26  
**Decided by:** TPM  
**Context:** Working name "RescueDisk" superseded by designer-proposed "FileResque" during Phase 0 design brief.  
**Options considered:**
- A: RescueDisk — descriptive, literal, less distinctive
- B: FileResque — punny portmanteau ("rescue" + "resque"), memorable, brand-friendly  
**Decision:** B — FileResque  
**Rationale:** Better brand recall; works as an app icon word-mark; the play on "rescue" communicates purpose without being clinical. Aligns with the high-end hardware tool aesthetic.  
**Consequences:** All agents must use "FileResque" in user-facing strings, docs, and bundle identifiers. Internal crate names (`rescuedisk-core` etc.) may retain working names until P5 packaging — update in P5-T01/T02.

---

## [DECISION-007] — Frontend Animation: GSAP (Approved External Dependency)

**Date:** 2026-06-26  
**Decided by:** TPM  
**Context:** Designer spec requires spring-physics button depressions and tension/release easing for neumorphic state transitions. CSS `transition` on `box-shadow` causes layout-thread jank.  
**Options considered:**
- A: CSS transitions only — zero JS dependency, but `box-shadow` transitions are notoriously janky and cannot express spring physics
- B: GSAP (GreenSock) — industry-standard animation library; ~30KB gzipped; handles `box-shadow` without jank via `gsap.to()`  
**Decision:** B — GSAP approved as a frontend dependency  
**Rationale:** Neumorphic design is the core visual language. Janky shadow transitions would undermine the "physical, tactile" feel that differentiates FileResque. GSAP is a mature, well-audited library with no network calls at runtime. This is a frontend JS dependency — does not affect the Rust minimum-deps philosophy.  
**Consequences:** `gsap` added to `package.json`. CSS `transition` is banned for interactive state changes (allowed only for non-interactive `opacity`). Developer agent must import GSAP in component setup and follow animation specs from designer.

---

## [DECISION-005] — Async Runtime: Tokio

**Date:** 2026-06-26  
**Decided by:** TPM  
**Context:** Disk scanning is long-running and must not block the UI thread.  
**Options considered:**
- A: Tokio — de facto async runtime; Tauri 2 uses it internally
- B: async-std — alternative runtime
- C: No async; use threads only
**Decision:** A — Tokio  
**Rationale:** Tauri 2 already depends on Tokio; using a second async runtime would cause conflicts. `tokio::spawn_blocking` cleanly handles CPU-bound disk parsing without blocking the async executor. Streaming results via `tokio::sync::mpsc` channels maps naturally to Tauri event emission.  
**Consequences:** All disk I/O code must be `spawn_blocking`-aware; pure async `await` on blocking syscalls is forbidden.

---

## [DECISION-008] — Single Colorway: Firmament Dark Palette Only (No Light Theme)

**Date:** 2026-06-26  
**Decided by:** User (recorded by TPM)  
**Context:** Phase 0 design brief (v1.0) included a full light-theme surface stack and light-theme text stack as a "system-preference fallback." This prompted review of whether maintaining two complete theme sets is justified for a v1 launch.  
**Options considered:**
- A: Ship both dark (Firmament) and light themes, with `prefers-color-scheme` media query switching
- B: Ship dark (Firmament) palette only; treat any future light-mode request as new scope
**Decision:** B — Single colorway, Firmament dark palette only  
**Rationale:** User-directed. Maintaining two complete, WCAG-audited, neumorphically-correct theme sets doubles the design surface and QA burden for no confirmed user requirement at v1. The Firmament palette was designed as the product's primary visual identity, not a fallback. Neumorphism in light mode requires a fully separate shadow calibration, which represents significant extra work.  
**Consequences:** The Designer is removing all light-theme material from `docs/design/design-brief.md` and `src/lib/styles/tokens.css`. Any `@media (prefers-color-scheme: light)` blocks are prohibited in production CSS. Future requests to add a light mode must be logged as Phase 6+ scope — they are not a bug fix or a minor addition.

---

## [DECISION-009] — Ratify `--color-text-dim` (#c8cfd9) as Supplementary Token

**Date:** 2026-06-26  
**Decided by:** TPM  
**Context:** The `designer.md` agent definition specifies a two-tier text stack: `--color-text-primary` (#f9f8f6) and `--color-text-secondary` (#8a94a6), plus `--color-text-disabled` (#4a5466). The Designer introduced a third mid-tier token `--color-text-dim: #c8cfd9` during the Phase 0 brief, positioned between primary and secondary, for use on card subtitles, disk names in lists, and icon labels that accompany visible text.  
**Options considered:**
- A: Ratify — keep the 3-tier stack (primary > dim > secondary > disabled)
- B: Reject — collapse to the agent-def 2-tier stack; any content needing a mid-brightness can use primary or secondary as-is
**Decision:** A — Ratify `--color-text-dim`  
**Rationale:** FileResque is a data-dense utility where typographic hierarchy directly affects scan-result legibility. The gap between #f9f8f6 (14.9:1) and #8a94a6 (5.1:1) on the dark surface is large; a disk name in a disk-list card is neither a headline nor a timestamp, and forcing it into one of those two buckets would flatten hierarchy that genuinely exists. The token is additive — it does not rename, shadow, or conflict with any agent-def token. Its contrast ratio (10.0:1 on `--color-bg-surface`) is WCAG AAA. The Designer has already referenced it in the icon-labelling rule in §8 and the deviation note in §3.  
**Deviation note:** This token is outside the pinned `designer.md` token set. It must be treated as a named extension, not a replacement. Component specs must still default to `--color-text-secondary` for secondary text; `--color-text-dim` is only appropriate where a sub-heading or prominent metadata item is semantically brighter than a supporting label but not a primary headline.  
**Consequences:** `--color-text-dim: #c8cfd9` is a ratified design token. Developer agent must include it in `src/lib/styles/tokens.css`. QA must verify contrast on all surfaces where it is used.

---

## [DECISION-010] — Titlebar Approach: Hybrid (Native Traffic Lights + Custom Title Area)

**Date:** 2026-06-26  
**Decided by:** TPM  
**Context:** Phase 0 design brief §11 raised a `[TPM_QUERY]` on whether to use (A) a fully custom neumorphic titlebar, (B) the native macOS transparent titlebar as-is, or (C) a hybrid — native traffic-light buttons with a custom title area overlaid. This decision affects the App shell in P0-T03 only.  
**Options considered:**
- A: Fully custom titlebar — maximum design control, full neumorphic integration, more implementation work, known edge cases with macOS window management (Mission Control thumbnails, Stage Manager, full-screen mode, system accessibility overrides)
- B: Native macOS transparent titlebar — stock traffic lights, no design control, simplest implementation
- C: Hybrid — native traffic-light buttons retained; standard titlebar hidden; custom overlay covers only the title/wordmark area
**Decision:** C — Hybrid  
**Rationale:** macOS users have strong muscle-memory expectations for traffic-light button placement and hit-targets. Reimplementing them introduces documented gotchas: hover state mismatches, full-screen transition edge cases, Stage Manager incompatibilities, and potential breakage on macOS point releases. The "macOS first" principle requires favouring native UX correctness. Option C satisfies both constraints: the native buttons remain functional and system-styled (handling accessibility preferences automatically), while the custom title area provides the FileResque wordmark and brand presence without touching window management primitives.  
**Consequences:** P0-T03 (App shell) implements `hiddenTitle: true` in `tauri.conf.json` with `titleBarStyle: "overlay"` on macOS. The drag region is declared via `data-tauri-drag-region` on the title area div. Windows secondary platform: native titlebar only (Option B equivalent), as the hybrid approach is macOS-specific. Developer agent must gate the title area render with a platform check.

---

## [DECISION-011] — Font Bundling: Bundle Both Inter and JetBrains Mono (woff2)

**Date:** 2026-06-26  
**Decided by:** TPM  
**Context:** Phase 0 design brief §11 raised a `[TPM_QUERY]` on whether to (A) bundle Inter + JetBrains Mono as woff2 (~240KB), (B) use system fonts only, or (C) bundle Inter only and use system monospace. HARD CONSTRAINT from CLAUDE.md: zero network calls — any web-font CDN import is unconditionally prohibited; woff2 bundled in the app binary is the only mechanism for non-system fonts.  
**Options considered:**
- A: Bundle both Inter (variable, ~120KB) + JetBrains Mono (subset ASCII + Latin Extended A, ~120KB) — consistent visual identity on macOS and Windows, no FOUT, fully offline
- B: System fonts only — zero bundle cost, but `system-ui` on macOS (SF Pro) vs Windows (Segoe UI) produces different metrics; system mono is Menlo (macOS) vs Consolas/Courier New (Windows), making cross-platform QA non-deterministic
- C: Bundle Inter only + system monospace — UI consistent, data display varies per platform
**Decision:** A — Bundle both Inter and JetBrains Mono  
**Rationale:** JetBrains Mono disambiguation is not merely aesthetic for FileResque — it is functional. All filenames, paths, inode IDs, sector addresses, and byte counts render in `--font-mono`. A user deciding whether to recover a file named `l1br4ry` vs `I1brary` in Menlo vs Consolas will get inconsistent character disambiguation across platforms. JetBrains Mono was specifically engineered for 0/O, 1/l/I disambiguation at small sizes, which aligns directly with the file-data display use case. The ~240KB bundle cost is negligible for a native desktop binary. Option C (Inter + system mono) is a false economy: it saves ~120KB but degrades QA reproducibility and creates a two-class user experience between macOS and Windows.  
**Consequences:** `src-tauri/` bundle includes `assets/fonts/Inter-variable.woff2` and `JetBrainsMono-subset.woff2`. `@font-face` declarations in `tokens.css` reference local paths only — no `url()` pointing to any external host. Font subsetting for JetBrains Mono: weight 400 + 500, ASCII + Latin Extended A only (covers all expected disk metadata). Developer agent must add font files to the Tauri bundle in `tauri.conf.json` `resources` array.

---

## [DECISION-012] — JS Toolchain: Bun + Biome (replaces pnpm + ESLint/Prettier)

**Date:** 2026-06-26
**Decided by:** User (recorded by TPM)
**Context:** P0-T01 scaffold was initially set up with pnpm as package manager and ESLint + Prettier for linting and formatting. User directed switch to Bun and Biome exclusively.
**Options considered:**
- A: pnpm + ESLint + Prettier — mature ecosystem, high configurability, three separate tools
- B: Bun + Biome — single fast runtime/package manager (Bun) + single fast linter+formatter (Biome), fewer config files
**Decision:** B — Bun + Biome
**Rationale:** User-directed. Biome replaces both ESLint and Prettier with a single, fast, zero-config-by-default tool. Bun replaces pnpm as the package manager and script runner. Reduces toolchain surface area.
**Consequences:** `package.json` scripts use `biome check`, `biome format`, `biome ci`. `Makefile` uses `bun` throughout. `tauri.conf.json` `beforeDevCommand`/`beforeBuildCommand` use `bun`. No `.eslintrc`, no `.prettierrc` — `biome.json` is the single JS/TS style config. `@biomejs/biome` is the only linting/formatting devDependency. All CI lint steps use `bun biome ci .`.