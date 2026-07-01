# FileResque — Brand Identity & Design System

**Version:** 2.0 (Rescue Beacon — full light palette revamp)
**Author:** Designer Agent
**Status:** DRAFT — Pending QA sign-off
**Tokens file:** `src/lib/styles/tokens.css`

**Changelog v1.2 → v2.0 (Rescue Beacon):**

- Full palette replacement: Firmament dark → Rescue Beacon light
- Token NAMES preserved identically across the codebase; only VALUES changed
- `--color-bg-base: #F7F8FA` (was #0a0e17 — matte warm white)
- `--color-bg-surface: #FFFFFF` (was #121826 — pure white card surface)
- `--color-accent-primary: #FFB020` (was #0084ff — amber beacon, no blue)
- `--color-text-primary: #1C2530` (was #f9f8f6 — charcoal ink on white)
- `--color-text-on-accent: #1C2530` (was #ffffff — white fails 1.83:1 on amber; ink = 8.46:1)
- Focus ring: `--color-focus-ring: #6D28D9` (was #ffc800 — deep mauve: 7.10:1 on white, 3.89:1 on amber)
- Shadow pair rebalanced for light neumorphism (translucent ink dark shadow, white light shadow)
- Tier tokens split into TEXT variants (dark, high-contrast) and BG tints
- Semantic warning text token: `--color-warning: #B45309` (amber-700, 5.02:1 on white)
- Scan shimmer: updated from blue to amber tint
- Brand personality updated: Firmament → Rescue Beacon (warm, clinical-clean, vector-art)

**Changelog v1.1 → v1.2:**

- Removed `### Light Theme (System-Preference Fallback)` section entirely (§3)
- Renamed `### Dark Theme (Hero — Default)` → `### The FileResque Palette`

**Changelog v1.0 → v1.1:**

- `--color-bg-base: #0a0e17` (was #0d1220); intermediate layer renamed `--color-bg-layer`
- Focus ring: `--color-warning` Construction Yellow (was blue); offset 2px
- `--radius-sm/md/lg: 4px/8px/12px` (were 6px/10px/16px)
- Added `--shadow-outset` / `--shadow-inset` canonical aliases

---

## 1. Brand Essence

### Positioning

FileResque is a power-user recovery instrument. It sits at the intersection of a surgical tool and a life raft. Unlike consumer "recovery wizard" software that infantilises the user, FileResque treats them as competent — they know what a filesystem is, they just need the job done, fast, correctly, and without compounding their already-bad day.

### Personality

| Trait | Expression |
|-------|------------|
| Calm authority | Typography is measured, never frantic. Space is generous. No blinking. |
| Technical precision | Monospace for all data: filenames, paths, sizes, inodes. Numbers are real. |
| Grounded warmth | Matte white surfaces and warm amber primary. Precise without feeling hostile. |
| Tactile responsiveness | Buttons depress physically. Progress is felt, not just seen. |
| Serious without cold | The palette is bright and clean, never clinical-grey. The primary action colour is amber — the beacon. |

### Emotional Target

The user opens FileResque in one of two states:

1. **Crisis mode** — a file they need is gone. Heart rate is elevated. The app must immediately project calm competence: "We have found the drive. Here is what is recoverable. Press this button."
2. **Maintenance mode** — a technician, power user, or IT admin running a routine scan. The app must reward attention to detail: precise metadata, clear probability tiers, no-nonsense workflow.

In both states, the design must do the same thing: **reduce cognitive load to zero for the primary action, and make secondary information discoverable but not intrusive.**

### Tone

- Direct. No wizard language ("Let's get started!"), no celebration emojis, no soft-pedalling bad news.
- Accurate. "3 of 7 files are unrecoverable" is a hard truth delivered clearly with a path forward.
- Grounded. The app behaves like a well-made physical tool: predictable, weighty, precise.

---

## 2. Logo & Wordmark Direction

### The Mark — Concept: "Lifeguard Disk"

The icon mark fuses two primary references:

1. **A document with a folded corner** — the universal file metaphor. Grounding the product in "files."
2. **A circular ring / lifebuoy halo** — the rescue metaphor. A thin-stroked ring encircles the document.

The resulting mark reads as: a document that has been saved, encircled by a ring of safety. At small sizes (dock icon, 16px favicon) the ring dominates; at medium sizes the folded document corner is clear.

```text
Icon construction (geometric):

        ╭────────────────╮
       ╱                  ╲        ← Lifeguard ring: thin stroke,
      │   ┌──────────┐     │         stroke-weight = 2px at 32px
      │   │          ├──╮  │       ← Document: rectangle with top-right
      │   │    FR    │  │  │         corner folded (45° triangle cut)
      │   │          ├──╯  │
      │   └──────────┘     │
       ╲                  ╱
        ╰────────────────╯

Construction notes:
- Ring is a perfect circle, NOT an oval. Stroke only, no fill.
- Document is centered within ring, occupying ~55% of ring diameter.
- Folded corner cut is exactly 1/4 of the shorter edge.
- A thin upward-pointing chevron (rescue arrow) sits inside the document,
  vertically centered, 30% of document height.
- Ring has two short gap breaks at 45° and 225° positions (NE and SW),
  suggesting an open circuit that has been "rescued"/closed.
```

### Colour Treatment

- **Ring:** `--color-accent-primary` (#FFB020 amber), full opacity stroke
- **Document body:** `--color-bg-surface` (#FFFFFF) fill, `--color-text-secondary` stroke at 1px
- **Folded corner:** `--color-tier-medium` (#92400E) — warm amber-brown accent, suggesting a recovered/flagged item
- **Rescue chevron inside document:** `--color-accent-primary` at 80% opacity

The mascot from the site (hooded rescuer with amber beacon face) is the brand's visual anchor. The icon mark simplifies the same concept: a glowing amber centre within a rounded-corner document shape.

### Wordmark

```text
  [ICON]  FileResque
          ──────────
          RECOVERY UTILITY

"File"    → Inter, weight 400 (Regular), --color-text-primary (#1C2530)
"Resque"  → Inter, weight 600 (SemiBold), --color-accent-primary (#FFB020)
Tagline   → Inter, weight 500 (Medium), 10px, tracked +0.12em,
            --color-text-secondary (#5B6675), uppercase
```

The deliberate misspelling "Resque" (vs "Rescue") is a feature, not a typo — it is domain-familiar (like "phreak", "3lite", "nix") and signals that this is a tool made by people who know their domain. Do not "correct" it.

### Minimum Sizes

- Icon: 16px minimum (ring drops to single-pixel, document corner hint only)
- Wordmark + icon together: 120px wide minimum
- Tagline only visible at 160px+ wide

---

## 3. Colour System

### Philosophy

The palette is structured in three layers:

1. **Rescue Beacon** — the bright, matte-white base that forms all surfaces. Named after the amber-faced mascot at the heart of the brand. Not pure clinical white (too cold) but a warm near-white (`#F7F8FA`) with slight warm undertones that reads "approachable precision tool." The signature amber (#FFB020) is the beacon glow — the single warm spot that draws the eye to the primary action.

2. **Warm Accents** — drawn from Catppuccin Latte. Each accent communicates a specific semantic meaning. No blue in any primary or accent role. Coral/peach for decorative fills; mauve for informational elements; amber for primary actions.

3. **Semantic Status** — colours tied to data states, mapping directly to `ProbabilityTier` (High/Medium/Low) and recovery outcomes. The distinction between FILL colours (used in backgrounds, graphical bars) and TEXT colours (darker variants safe for body text on white) is critical on light surfaces — pastels that work as fills often fail as body text.

### The FileResque Palette

```text
RESCUE BEACON SURFACE STACK (bottom layer to top):

  #E4E8EE  bg-void    — window chrome hairline border (deepest contrast element)
  #F7F8FA  bg-base    — app shell background  ← CANONICAL (matte warm white)
  #F0F2F6  bg-layer   — intermediate panel/content area background
  #FFFFFF  bg-surface — primary panel/card surface  ← NEUMORPHIC BASE (pure white)
  #F0F2F6  surface-2  — elevated within surface (inputs, nested cards)
  #E4E8EE  surface-3  — border/divider layer (dropdown bg, tooltip bg)
  #D6DBE3  surface-4  — active/hover highlight layer
```

Shadow pair for light neumorphism on `bg-surface` (#FFFFFF):

- Dark shadow: `rgba(28, 37, 48, 0.12)` — translucent ink drop, cast bottom-right
- Light shadow: `rgba(255, 255, 255, 0.90)` — near-white highlight, top-left

On light surfaces, the dark shadow reads as a soft cast shadow and the white highlight bleaches the top-left corner — creating physical depth without dark gutters.

```text
WARM ACCENTS (no blue anywhere):

  #FFB020  amber-beacon   — primary safe action CTA (fill only, NOT standalone text)
  #5B21B6  mauve-info     — informational tooltips, data labels (8.98:1 on white — text safe)
  #FF8A3D  coral-alt      — sparklines, audit markers, decorative fills (NOT text on white)
```

```text
SEMANTIC COLOURS:

  Fill colours (for backgrounds, badges, graphical elements — NOT body text on white):
  #2F9E5F  success fill   — High probability, recovered indicator
  #FFB020  warning fill   — Medium probability (same as amber-beacon primary)
  #C0392B  danger fill    — Low probability, critical errors, destructive actions

  Text colours (darker variants for body text directly on bg-surface / bg-base):
  #166534  tier-high text   — 7.13:1 on white  → body text PASS
  #92400E  tier-medium text — 7.09:1 on white  → body text PASS
  #991B1B  tier-low text    — 8.31:1 on white  → body text PASS
  #B45309  warning text     — 5.02:1 on white  → body text PASS
  #C0392B  danger text      — 5.44:1 on white  → body text PASS
```

```text
TEXT STACK:

  #1C2530  text-primary   — headlines, file names, primary UI labels  (15.48:1 on white)
  #374151  text-dim       — sub-headings, prominent metadata          (10.31:1 on white)
  #5B6675  text-secondary — supporting labels, timestamps             (5.83:1 on white)
  #9AA3AE  text-disabled  — explicitly disabled states only           (2.55:1 on white, intentional)
  #1C2530  text-on-accent — text inside amber primary button          (8.46:1 on #FFB020)
  #1C2530  text-on-warning — text on amber-tinted callout surfaces    (8.46:1 on #FFB020)
```

Note: `--color-text-dim` (#374151) is a supplementary token between primary and secondary. Component specs should default to `text-secondary` for secondary text; `text-dim` is available where a stronger mid-tone is needed (card subtitles, disk names in the list).

### Accessibility Contrast Audit

All values measured against WCAG 2.1 AA (4.5:1 normal text, 3:1 large/bold text ≥ 18px or ≥ 14px/bold). Primary background is `--color-bg-surface` (#FFFFFF). Ratios confirmed with the WCAG relative-luminance formula.

| Foreground | Token | Background | Ratio | WCAG AA Normal | WCAG AA Large |
|------------|-------|------------|-------|----------------|---------------|
| `#1C2530` | text-primary | `#FFFFFF` surface | 15.48:1 | PASS | PASS |
| `#374151` | text-dim | `#FFFFFF` surface | 10.31:1 | PASS | PASS |
| `#5B6675` | text-secondary | `#FFFFFF` surface | 5.83:1 | PASS | PASS |
| `#9AA3AE` | text-disabled | `#FFFFFF` surface | 2.55:1 | FAIL (intentional — disabled) | FAIL (intentional) |
| `#1C2530` | text-primary | `#F7F8FA` bg-base | 14.57:1 | PASS | PASS |
| `#5B6675` | text-secondary | `#F7F8FA` bg-base | 5.49:1 | PASS | PASS |
| `#1C2530` on `#FFB020` | text-on-accent | amber primary fill | 8.46:1 | PASS | PASS |
| `#FFFFFF` on `#FFB020` | (rejected) | amber primary fill | 1.83:1 | FAIL — DO NOT USE | FAIL |
| `#166534` | tier-high text | `#E6F6EE` tier-high-bg | 6.38:1 | PASS | PASS |
| `#92400E` | tier-medium text | `#FEF3E0` tier-medium-bg | 6.45:1 | PASS | PASS |
| `#991B1B` | tier-low text | `#FDEAEA` tier-low-bg | 7.18:1 | PASS | PASS |
| `#2F9E5F` | success fill as text | `#FFFFFF` surface | 3.40:1 | FAIL normal | PASS large only |
| `#B45309` | warning text | `#FFFFFF` surface | 5.02:1 | PASS | PASS |
| `#C0392B` | danger text | `#FFFFFF` surface | 5.44:1 | PASS | PASS |
| `#5B21B6` | accent-info text | `#FFFFFF` surface | 8.98:1 | PASS | PASS |
| `#6D28D9` | focus ring | `#FFFFFF` surface | 7.10:1 | PASS | PASS |
| `#6D28D9` | focus ring | `#FFB020` amber fill | 3.89:1 | FAIL normal | PASS large (≥ 3:1) |
| `#6D28D9` | focus ring | `#F7F8FA` bg-base | 6.69:1 | PASS | PASS |

**Critical constraint — amber primary button text:** White on amber (#FFB020) = 1.83:1 — fails every WCAG threshold. `--color-text-on-accent` is `#1C2530` (ink), not white. This is a hard rule enforced in Button.svelte. The ink/amber pair achieves 8.46:1 — normal text passes at any size.

**Success fill as body text warning:** `#2F9E5F` (success fill) achieves only 3.40:1 on white — passes AA Large but fails AA Normal. The `--color-success` fill token must never be used as standalone body text on white surfaces. Badge.svelte uses `--color-tier-high` (#166534) for badge text colour, which passes at 6.38:1 on the tinted badge background. Standalone semantic text (e.g. a "Recovered" label in a detail pane) must use `--color-tier-high` (#166534), not `--color-success`.

**Coral accent (`#FF8A3D`) as text warning:** Coral achieves 2.35:1 on white — fails all thresholds. `--color-accent-alt` (#FF8A3D) is a FILL-ONLY token: graphical bars, sparklines, decorative chips. Never use as text on white.

**Focus ring choice — Deep Mauve `#6D28D9`:** Construction Yellow (#FFC800) from the Firmament era achieved 9.8:1 on dark backgrounds but only ~1.9:1 on white — invisible on the new light surfaces. Deep mauve `#6D28D9` achieves 7.10:1 on white (strong PASS), 3.89:1 on the amber primary fill (PASS large), and 6.69:1 on bg-base. It is visually distinct from text ink (charcoal) and from amber — no perceptual confusion. Catppuccin Latte's own mauve (#8839EF) was rejected because it achieves only 2.96:1 on amber (below the 3:1 minimum).

**Neumorphic surface contrast note:** Light neumorphism creates depth through a translucent ink drop shadow (bottom-right) and a white highlight (top-left). The focus ring (2px deep mauve outline, 2px offset) is mandatory on all interactive elements. Neumorphic depth cues alone are insufficient to indicate interactivity, particularly for keyboard users and in bright ambient light conditions.

---

## 4. Typography

### Type Families

**Primary UI Font: Inter**
Rationale: Humanist grotesque with exceptional legibility at 12–14px (critical for data-dense tables). Excellent Unicode coverage for cross-platform file names. Free, variable font available for weight-interpolation without flash of unstyled text.

```
@import url from local or bundle — NO network calls (Tauri offline constraint)
Files ship with app bundle: Inter-variable.woff2 (covers weight 100–900)
```

**Monospace Accent Font: JetBrains Mono**
Rationale: Designed specifically for code/data display. Excellent disambiguation (0/O, 1/l/I). Used exclusively for: file names, paths, inode IDs, byte counts, hex values, disk identifiers (e.g. "disk0s2"). Ships as static subset (ASCII + Latin Extended A) to keep bundle size minimal.

```
Weight subset needed: 400, 500 — no bold needed in mono contexts
```

Token names: `--font-family` (UI sans, canonical per agent-def), `--font-sans` (extended alias), `--font-mono` (monospace, canonical per agent-def).

### Type Scale

Base is 14px (not 16px) — this is a data-dense utility, and 14px is standard for professional desktop tools (Figma, VS Code, Terminal).

```
--font-size-2xs:  10px   (badge labels, very small metadata — use sparingly)
--font-size-xs:   11px   (table column headers, timestamps, secondary mono data)
--font-size-sm:   12px   (table cell text, secondary labels, monospace paths)
--font-size-md:   14px   (body default, primary labels, button text)
--font-size-lg:   16px   (card titles, section sub-headings, modal body)
--font-size-xl:   20px   (section headings, modal titles)
--font-size-2xl:  24px   (page-level headings)
--font-size-3xl:  32px   (splash/onboarding hero text)
```

### Font Weight Usage

```
400  Regular     — body text, table cells, descriptions
500  Medium      — column headers, labels, navigation items
600  SemiBold    — card titles, button text, file names in results
700  Bold        — section headings, modal titles, status callouts
```

### Line Heights

```
--line-height-none:     1.0   (single-line badges, chips — prevent clipping)
--line-height-tight:    1.2   (headings)
--line-height-snug:     1.35  (mono data rows, table cells)
--line-height-base:     1.5   (body text)
--line-height-relaxed:  1.75  (help text, descriptions in modals)
```

### Letter Spacing

```
--tracking-tight:   -0.01em  (headings at large sizes)
--tracking-normal:   0       (body default)
--tracking-wide:    +0.04em  (uppercase labels, tagline, column headers)
--tracking-wider:   +0.08em  (all-caps badge text)
```

### Usage Rules

1. All file names from disk (including in tables and detail panels) render in `--font-mono`. Always. They are data, not UI labels.
2. Byte counts, sizes, inode IDs, sector addresses: `--font-mono`, `--font-size-sm`.
3. Progress percentages and transfer rates: `--font-mono`, tabular-nums variant: `font-variant-numeric: tabular-nums` — this prevents the progress bar numbers from jumping in width as digits change.
4. Never use `text-transform: uppercase` on user-generated file names — they come from disk metadata and must be rendered as-is.
5. Maximum line length for body/description text: 72ch. Beyond this, readability degrades on wide screens.

---

## 5. Elevation & Neumorphism System

### Principle

Neumorphism is applied to **surfaces and controls only** — not to text, icons, or semantic status elements. The light source is top-left (convention: 315° / northwest). The shadow system has three states:

| State | Name | Usage |
|-------|------|-------|
| Raised | `--shadow-outset` | Default resting state: cards, buttons, inputs |
| Flat | `--shadow-flat` | Background-flush elements, table rows, toolbar items |
| Pressed / Inset | `--shadow-inset` | Active button, selected list item, depressed toggle |

### Shadow Tokens

Two tiers of shadow tokens exist in `tokens.css`:

**Canonical names (use in GSAP code and component specs):**

```css
--shadow-outset: 4px 4px 10px rgba(28, 37, 48, 0.12), -4px -4px 10px rgba(255, 255, 255, 0.90)
--shadow-inset:  inset 4px 4px 10px rgba(28, 37, 48, 0.12), inset -4px -4px 10px rgba(255, 255, 255, 0.90)
```

These are the values the agent-def Design Token Format pins. GSAP button animations call `var(--shadow-inset)` and `var(--shadow-outset)` directly — do not hardcode the pixel values in JS. Shadow colours are now rgba ink + white (Rescue Beacon light surface); the dark `#07090f`/light `#1d273d` pair from Firmament is retired. Note: CSS custom properties do not resolve rgba() inside compound shadow compound values via var() in all older engines; the rgba literals are embedded directly in the token definitions.

**Levelled variants (for components needing more or less depth than standard):**

```text
--shadow-raised-sm   6px 6px 12px ...   — table row hover, nested cards
--shadow-raised-md   8px 8px 18px ...   — heavier card/panel raise
--shadow-raised-lg   12px 12px 28px ...  — modals, floating dropdowns
--shadow-inset-sm    inset 3px 3px 6px  — selected row, active tab
--shadow-inset-md    inset 5px 5px 12px — active input, pressed card
```

### Interaction State Mapping

```text
Button:
  Default   → var(--shadow-outset)
  Hover     → var(--shadow-outset) + translate(-1px, -1px), scale(1.005) — via GSAP
  Active    → var(--shadow-inset), translate(0,0), scale(1.0) — via GSAP
  Focus     → var(--shadow-outset) + 2px solid outline var(--color-focus-ring) (#6D28D9), offset 2px
  Disabled  → var(--shadow-flat), opacity 0.38, cursor not-allowed

Input / Select:
  Default   → var(--shadow-inset-sm) (inputs are recesses in the surface)
  Focus     → var(--shadow-inset-sm) + 1px solid var(--color-accent-primary) border
  Error     → var(--shadow-inset-sm) + 1px solid var(--color-danger) border
  Disabled  → var(--shadow-flat), opacity 0.38

Card / Panel:
  Default   → var(--shadow-outset)
  Hover     → var(--shadow-outset) + 1px solid var(--color-bg-surface-3) border
  Selected  → var(--shadow-outset) + 1px solid var(--color-accent-primary) border + left accent bar

Table Row:
  Default   → var(--shadow-flat)
  Hover     → var(--shadow-raised-sm)
  Selected  → var(--shadow-inset-sm) + var(--color-accent-primary) left border bar 2px
```

### Border Radius Scale

Per agent-def Design Token Format (sm/md/lg/pill are canonical; xs and xl are additive):

```text
--radius-none:  0px
--radius-xs:    2px    (micro rounding — additive)
--radius-sm:    4px    (chips, badges, small indicators — canonical)
--radius-md:    8px    (standard buttons, cards, panels — canonical)
--radius-lg:    12px   (large panels, modals — canonical)
--radius-xl:    20px   (hero cards, onboarding surfaces — additive)
--radius-pill:  9999px (pill badges, toggle switches — canonical)
```

Consistent rule: **the inner element radius = outer radius minus padding.** A button (radius-md = 8px) that contains an icon badge (padding 3px) means the icon badge uses radius 5px — this preserves the concentric-ring look fundamental to the neumorphic aesthetic.

---

## 6. Spacing, Z-Index & Motion Tokens

### Spacing (4px base grid)

```
--space-px:   1px
--space-0-5:  2px
--space-1:    4px
--space-2:    8px
--space-3:    12px
--space-4:    16px
--space-5:    20px
--space-6:    24px
--space-8:    32px
--space-10:   40px
--space-12:   48px
--space-16:   64px
--space-20:   80px
--space-24:   96px
```

All component internal padding and margin must be a multiple of 4px. No arbitrary pixel values.

### Z-Index Stack

```
--z-base:      0    (normal document flow)
--z-raised:    10   (raised cards, sticky table headers)
--z-dropdown:  100  (dropdown menus, autocomplete)
--z-overlay:   200  (modals, drawers)
--z-toast:     300  (notification toasts)
--z-tooltip:   400  (tooltips — above everything)
```

### Motion Tokens

FileResque uses GSAP for all animated interactive states. CSS `transition` is permitted only for `opacity` on non-interactive elements (e.g. tooltips). Never use CSS `transition` on `box-shadow` — it causes perceptible jank. GSAP drives all shadow and transform animations.

**Duration:**

```text
--duration-instant:    0ms    (reduced-motion fallback)
--duration-fastest:    80ms   (micro-interactions: icon swap, badge colour change)
--duration-fast:       140ms  (button press state, row hover)
--duration-base:       240ms  (panel reveal, modal appear)
--duration-slow:       400ms  (page-level transitions, onboarding steps)
--duration-crawl:      800ms  (progress bar smooth-fill easing)
```

**Easing (CSS cubic-bezier equivalents — GSAP uses these same curves):**

```text
--ease-spring:    cubic-bezier(0.34, 1.56, 0.64, 1)
  → Spring with slight overshoot. Used for: button release, card hover lift,
    modal entrance. The 1.56 Y value creates the tactile "pop".

--ease-snappy:    cubic-bezier(0.4, 0, 0.2, 1)
  → Material Design standard. Used for: colour transitions, badge updates,
    progress bar fill. No overshoot — appropriate for data state changes.

--ease-depress:   cubic-bezier(0.36, 0.07, 0.19, 0.97)
  → Heavy press-in curve — slow start, abrupt end. Used exclusively for
    the GSAP button active/press state. Simulates real button mechanics.

--ease-out-cubic: cubic-bezier(0.33, 1, 0.68, 1)
  → Clean exit. Used for: toast dismiss, modal close, row remove animation.
```

**GSAP Configuration Templates (canonical from agent-def):**

```javascript
// Button press — spring physics
// Uses var(--shadow-inset) and var(--shadow-outset) canonical token names.
gsap.to(button, {
  boxShadow: 'var(--shadow-inset)',
  duration: 0.12,
  ease: 'power2.in',        // fast compress
  onComplete: () =>
    gsap.to(button, {
      boxShadow: 'var(--shadow-outset)',
      duration: 0.35,
      ease: 'elastic.out(1, 0.4)',  // spring release
    })
});

// Progress bar fill — no bounce
gsap.to(progressBar, { width: `${pct}%`, duration: 0.4, ease: 'power1.out' });

// Panel reveal — tension ease
gsap.fromTo(panel, { opacity: 0, y: 8 }, { opacity: 1, y: 0, duration: 0.25, ease: 'power2.out' });
```

**Reduced Motion:**
All GSAP animations must check `window.matchMedia('(prefers-reduced-motion: reduce)')`. If true: durations collapse to `--duration-instant`, no transforms — only opacity fades at `--duration-fast`.

---

## 7. Component Inventory

These are the components required for the full app (P0–P4). Each will receive a full spec in the corresponding task planning doc under `## Designer Spec`. The planning doc for P0-T03 is `docs/agent-planning/03-p0-design-system.md` — to be created when that task starts.

| Component | Phase | Purpose |
|-----------|-------|---------|
| `<Button>` | P0-T03 | Primary, secondary, ghost, danger variants |
| `<Card>` | P0-T03 | Raised neumorphic container |
| `<Badge>` | P0-T03 | ProbabilityTier, filesystem type, status labels |
| `<ProgressBar>` | P0-T03 | Scan progress, recovery progress, block fill indicator |
| `<Modal>` | P0-T03 | Overlay for recovery confirmation, permission onboarding |
| `<DiskList>` | P1-T03 | Disk enumeration results; selectable disk cards |
| `<DiskCard>` | P1-T03 | Single disk: icon, name, size, filesystem badge, encryption lock |
| `<FileTable>` | P2-T05 | Virtualised 10k+ row table for deleted file results |
| `<FileRow>` | P2-T05 | Single deleted file row: icon, name, size, date, probability |
| `<ProbabilityPanel>` | P3-T02 | Inline expandable: tier badge, block breakdown, warnings |
| `<RecoveryModal>` | P4-T03 | Confirmation + progress + completion states |
| `<PermissionGate>` | P1-T04 | Full-screen onboarding for FDA permission |
| `<Toast>` | P5-T03 | Non-blocking notifications: success, warning, error |
| `<ErrorBoundary>` | P5-T03 | Svelte error boundary with graceful fallback UI |

---

## 8. Iconography & Texture Guidance

### Icon Style

Use a single, consistent icon library. Recommendation: **Lucide Icons** (MIT licence, SVG, tree-shakeable, actively maintained, excellent at small sizes).

Rules for all icons in FileResque:

1. **Stroke width: 1.5px at 16px, 1.75px at 20px, 2px at 24px.** Never fill icons. The line-art style reads as precise/technical vs the blunt-instrument feel of filled icons.
2. **Colour:** Default icon colour is `--color-text-secondary` (#5B6675). Interactive/labelled icons (with visible text label alongside) use `--color-text-dim` (#374151). Semantic icons (success/warning/danger) use their respective accent colour.
3. **Size: use 16px or 20px.** 24px only for empty states and large contextual icons.
4. **Never scale icons with CSS `transform`** — always use a new size. Fractional pixel rendering destroys the stroke precision.

### Specific Icon Assignments

```text
HDD:        hard-drive       (Lucide)
SSD / NVMe: database         (Lucide) — abstracted storage
USB:        usb              (Lucide, or usb-2 if available)
Lock:       lock             (Lucide — encrypted volume)
Unlock:     lock-open        (Lucide)
File:       file             (Lucide)
Image:      image            (Lucide)
PDF:        file-text        (Lucide)
Archive:    archive          (Lucide)
Video:      video            (Lucide)
Unknown:    file-question    (Lucide)
Scan:       scan-line        (Lucide)
Recovery:   arrow-up-from-dot (Lucide — rescue metaphor)
Warning:    triangle-alert   (Lucide)
Error:      circle-x         (Lucide)
Success:    circle-check     (Lucide)
Info:       info             (Lucide)
Spinner:    loader-circle    (Lucide — animated with CSS rotation)
Settings:   settings-2       (Lucide)
Folder:     folder-open      (Lucide — destination picker)
Cancel:     x                (Lucide)
```

### Texture & Surface Treatment

Neumorphic surfaces should be **perfectly smooth** — no grain, no noise, no glassmorphism blur. The depth effect comes entirely from precise shadow offsets.

**One exception:** The app titlebar / window chrome can use a very subtle Perlin-noise grain at 3% opacity (not visible, but prevents the large flat surface from looking "digital" or "flat-flat"). This is optional and only applied to `--color-bg-base` (#F7F8FA) areas.

**Scan progress overlay:** During an active scan, a subtle animated gradient sweep (1–2% opacity, CSS `@keyframes`) can move across the scan panel's surface, suggesting electromagnetic activity. This is purely atmospheric — it must not interfere with readability and must stop when `prefers-reduced-motion` is set.

```css
/* Scan activity shimmer — atmospheric only */
@keyframes scan-sweep {
  0%   { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}
.scanning {
  background-image: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 176, 32, 0.06) 50%,
    transparent 100%
  );
  background-size: 200% 100%;
  animation: scan-sweep 3s linear infinite;
}
@media (prefers-reduced-motion: reduce) {
  .scanning { animation: none; }
}
```

---

## 9. Layout Principles

### App Shell

```text
┌──────────────────────────────────────────────────────┐
│  TITLEBAR (Tauri custom, drag region, 36px tall)     │
│  [icon] FileResque              [─][□][×]            │
├──────────────────────────────────────────────────────┤
│  SIDEBAR (220px, collapsible)  │  MAIN CONTENT AREA  │
│  ─────────────────             │  (flex-1, scrollable│
│  Disk: disk0 (Macintosh HD)    │   per-route)        │
│  ─────────────────             │                     │
│  [Scan]                        │                     │
│  [Results]                     │                     │
│  [Recovery Log]                │                     │
│  ─────────────────             │                     │
│  [Settings]                    │                     │
├──────────────────────────────────────────────────────┤
│  STATUS BAR (28px, fixed bottom)                     │
│  disk0 · APFS · 512 GB · TRIM enabled    Ready  ●   │
└──────────────────────────────────────────────────────┘
```

### Responsive Behaviour

This is a desktop-only app (Tauri, not browser). Minimum window size: 900×600px. Design target: 1280×800px. Sidebar collapses to icon-only at window widths below 960px. The `<FileTable>` takes all available remaining height.

### Progressive Disclosure

Phase order maps directly to UI reveal order:

1. **Start:** Only disk list is visible. No table, no progress, no recovery. One primary action: "Select a disk."
2. **Scanning:** Disk list collapses to selected-disk summary chip. Scan progress bar + live file count appears. Cancel button visible.
3. **Results:** File table appears. Probability badges are "?" until clicked. Single primary action: "Recover Selected."
4. **Recovery:** Modal overlays results. Can't accidentally trigger scan again.

This phased reveal prevents the user from doing the wrong thing (e.g., recovering to the same disk they're scanning).

---

## 10. Process Notes

**Planning doc:** This document is the Phase 0 chat-first brand identity consultation that precedes P0-T03. The formal planning document for P0-T03 (`docs/agent-planning/03-p0-design-system.md`) is to be created by the agent running that implementation task. The `[TPM_QUERY]` items below should be cross-posted to that planning doc when P0-T03 is kicked off.

**Status:** This brief remains DRAFT until TPM resolves the open queries below and QA signs off on the initial token implementation in P0-T03.

---

## 11. Component Migration Notes (Rescue Beacon v2.0)

The token swap is automatic for all components that use only `var(--*)` references. The items below require **bespoke developer attention** beyond the token update — either because they contain hardcoded values, reference semantics that changed meaning, or need visual re-verification on the new light surface.

### Items requiring developer action

**1. `Modal.svelte` — backdrop colour**

Current: `color-mix(in srgb, var(--color-bg-void) 85%, transparent)`

On Firmament, `--color-bg-void` was near-black (`#070912`) so the backdrop was a near-opaque dark scrim. On Rescue Beacon, `--color-bg-void` is `#E4E8EE` (a light grey). The formula now produces a light translucent overlay — which is too weak to dim the content behind a modal.

Required change: Update the backdrop to use the ink colour instead, e.g.
`color-mix(in srgb, var(--color-text-primary) 55%, transparent)` or a fixed `rgba(28, 37, 48, 0.50)`. The designer recommendation is `rgba(28, 37, 48, 0.45)` — enough scrim to de-emphasise background content without feeling oppressive on a light UI.

**2. `+page.svelte` — scan-error banner background**

Current: `color-mix(in srgb, var(--color-danger) 8%, transparent)`

`--color-danger` was a saturated red on dark (#e5534b). The new value is a darker crimson (#C0392B). At 8% on white, both produce a very faint pink tint. This will render correctly but should be verified visually — the tint may need to increase to 10–12% on a light surface to remain perceptible as an error state.

**3. `Badge.svelte` — `badge--info` background**

Current: `color-mix(in srgb, var(--color-accent-info) 12%, transparent)`

`--color-accent-info` changed from soft blue (#58a6ff) to deep mauve (#5B21B6). The 12% mix on white produces a correct light-mauve tint. Visual verification recommended — the mauve tint at 12% may be too subtle; consider raising to 15% if the badge does not read clearly.

**4. `RecoveryModal.svelte` — status row backgrounds**

Current uses `color-mix(in srgb, var(--color-danger) 8%, transparent)` and `color-mix(in srgb, var(--color-success) 8%, transparent)`.

Both danger and success token values changed. The percentage should remain functional; verify the tints read clearly on the white modal surface.

**5. Titlebar / app shell (`+page.svelte`)**

The titlebar and both `panel--left` / `panel--right` use `background-color: var(--color-bg-base)`. On Firmament this was near-black; on Rescue Beacon it is `#F7F8FA`. The border `1px solid var(--color-border)` was a white-alpha; it is now an ink-alpha (`rgba(28, 37, 48, 0.11)`). Both will invert automatically via the token swap and should render correctly. Verify the titlebar `border-bottom` is visible — a single-pixel ink-alpha divider may be very faint on the warm-white base.

#### 6. Wordmark "Resque" accent colour

In the titlebar wordmark, `.wordmark strong { color: var(--color-accent-primary) }` now renders amber (#FFB020). Amber text on white achieves 1.83:1 — this FAILS WCAG for body text. However, the wordmark "Resque" is rendered at `font-size-md` (14px) weight-600. Under WCAG large text rules (≥ 14px bold = 3:1 minimum), 1.83:1 still fails.

Required action: Change the wordmark accent to `var(--color-tier-medium)` (#92400E, amber-700, 7.09:1 on white) for the in-app titlebar. The amber fill (#FFB020) remains correct for graphical icon elements. The site can continue using amber decoratively for the wordmark since the site is not governed by app WCAG requirements; the app titlebar text must use the darker variant.

**7. `.scanning` shimmer (tokens.css)**

Updated from `rgba(0, 132, 255, 0.04)` blue to `rgba(255, 176, 32, 0.06)` amber. This is already applied in tokens.css. Developer should verify the shimmer is perceptible on `--color-bg-surface` (#FFFFFF) — 6% amber on pure white is very subtle. If imperceptible, raise opacity to 0.08.

**8. Scrollbar colours (`app.css`)**

The scrollbar track uses `var(--color-bg-layer)` (#F0F2F6) and thumb uses `var(--color-bg-surface-3)` (#E4E8EE). These invert automatically via tokens. Verify the thumb (#E4E8EE) is perceptible on the `#F0F2F6` track — the contrast between these two light greys may be too low to distinguish. If the scrollbar is invisible, the thumb should use `var(--color-border-default)` (#D6DBE3 approx) instead.

#### 9. Icon colours

Icons default to `--color-text-secondary`. On Firmament this was `#8a94a6` (mid-grey on dark). On Rescue Beacon it is `#5B6675` (5.83:1 on white) — correctly readable. No changes needed. Semantic icons (success/warning/danger) still use their respective tokens, which now resolve to the darker text-safe variants (#166534, #92400E, #991B1B / #C0392B). These are correct for icon fill on white surfaces.

**10. `--color-selection-bg` change**

Changed from `rgba(0, 132, 255, 0.15)` (blue selection) to `rgba(255, 176, 32, 0.18)` (amber selection). Any component using CSS `::selection` or applying this token for row/text selection highlight should verify the amber tint is readable. Selected text on an amber-tinted selection highlight: `--color-text-primary` (#1C2530) on amber tint remains readable.

### Hardcoded values outside tokens.css — audit result

A grep of `src/` for hex literals and `rgba()` outside `tokens.css` found only `color-mix()` expressions referencing CSS tokens (no raw hex), `background: transparent`, and `background: none`. No hardcoded colour hex values exist in any `.svelte` or `.css` file outside `tokens.css`. The codebase is clean — the token swap propagates completely.

---

## 12. Open Questions for TPM

```text
[TPM_QUERY]
From: Designer
Phase: P0
Task: design-brief
Question: The Tauri window uses a custom titlebar. Should we design a fully
  custom titlebar (draggable region, custom min/max/close buttons) or use
  the native macOS titlebar in transparent mode?
Options:
  [A] Fully custom titlebar — maximum design control, full neumorphic integration,
      more implementation work, potential edge case bugs with macOS window management
  [B] Native macOS titlebar (transparent) — system-native UX, less design control,
      simpler implementation, traffic light buttons remain stock
  [C] Hybrid: native traffic-light buttons + custom title area (hidden standard titlebar,
      overlay the title area only)
Blocking: no — P0-T03 can proceed either way; decision affects App.svelte shell only
```

```text
[TPM_QUERY]
From: Designer
Phase: P0
Task: design-brief
Question: Should the app ship Inter and JetBrains Mono as bundled font files
  (woff2, ~120KB each) or use system fonts as fallback (system-ui + monospace)?
Options:
  [A] Bundle both fonts — consistent visual identity across macOS/Windows,
      larger app bundle, no FOUT, fully offline (Tauri constraint satisfied)
  [B] System fonts only — zero bundle cost, native feel, but loses the precise
      visual identity (especially JetBrains Mono vs Menlo/Consolas differences)
  [C] Bundle Inter only, use system monospace — compromise: UI is consistent,
      data display adapts to platform
Blocking: no — tokens are written with the font-family stack either way
```
