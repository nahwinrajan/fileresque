# FileResque — Brand Identity & Design System

**Version:** 1.2 (stripped all light-theme material — single colorway)
**Author:** Designer Agent
**Status:** DRAFT — Pending TPM review and QA sign-off
**Tokens file:** `src/lib/styles/tokens.css`

**Changelog v1.1 → v1.2:**
- Removed `### Light Theme (System-Preference Fallback)` section entirely (§3)
- Renamed `### Dark Theme (Hero — Default)` → `### The FileResque Palette`
- Removed light-theme logo fallback note from §2 Colour Treatment
- Removed light-theme focus ring override sentence from §3 Accessibility Contrast Audit
- Removed "overridden per theme / update automatically on theme switch" from §5 Shadow Tokens
- Removed all "hero", "default theme", and theme-switching framing from prose

**Changelog v1.0 → v1.1:**
- `--color-bg-base: #0a0e17` (was #0d1220); intermediate layer renamed `--color-bg-layer`
- `--color-shadow-light: #1d273d` (was #1d2739, one-digit typo)
- `--color-text-secondary: #8a94a6` (canonical per agent-def); #c8cfd9 renamed `--color-text-dim`
- Focus ring: `--color-warning` Construction Yellow (was blue); offset 2px (was 3px)
- `--radius-sm/md/lg: 4px/8px/12px` (were 6px/10px/16px)
- Added `--shadow-outset` / `--shadow-inset` canonical aliases (required by GSAP templates)
- Added `--font-family` canonical token name
- Status downgraded from "APPROVED" to "DRAFT" (no QA sign-off has occurred)
- Typo fix: "lifeguoy" → "lifebuoy"

---

## 1. Brand Essence

### Positioning

FileResque is a power-user recovery instrument. It sits at the intersection of a surgical tool and a life raft. Unlike consumer "recovery wizard" software that infantilises the user, FileResque treats them as competent — they know what a filesystem is, they just need the job done, fast, correctly, and without compounding their already-bad day.

### Personality

| Trait | Expression |
|-------|------------|
| Calm authority | Typography is measured, never frantic. Space is generous. No blinking. |
| Technical precision | Monospace for all data: filenames, paths, sizes, inodes. Numbers are real. |
| Grounded confidence | Deep, stable background palette. The app does not vibrate with anxiety. |
| Tactile responsiveness | Buttons depress physically. Progress is felt, not just seen. |
| Serious without cold | The palette is deep and stable, never hostile. The primary action colour is blue — safe, trusted. |

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

```
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

- **Ring:** `--color-accent-primary` (#0084ff), full opacity
- **Document body:** `--color-bg-surface` (#121826) fill, `--color-accent-primary` stroke at 1px
- **Folded corner:** `--color-warning` (#ffc800) — a small gold accent, suggesting a recovered/flagged item
- **Rescue chevron inside document:** `--color-accent-primary` at 80% opacity

### Wordmark

```
  [ICON]  FileResque
          ──────────
          RECOVERY UTILITY

"File"    → Inter, weight 400 (Regular), --color-text-primary
"Resque"  → Inter, weight 600 (SemiBold), --color-accent-primary
Tagline   → Inter, weight 500 (Medium), 10px, tracked +0.12em,
            --color-text-secondary (#8a94a6), uppercase
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

1. **Firmament** — the dark blue-black base that forms all surfaces. Named after the deep sky: stable, infinite, trustworthy. Not pure black (too harsh) but a deeply saturated near-black with blue undertones that reads "precision instrument."

2. **Cyber Accents** — electric, high-purity hues used sparingly. Each accent communicates a specific semantic meaning and is never used decoratively.

3. **Semantic Status** — colours tied to data states. These map directly to `ProbabilityTier` (High/Medium/Low) and recovery outcomes (success, warning, danger).

### The FileResque Palette

```
FIRMAMENT SURFACE STACK (bottom to top):

  #070912  bg-void    — window chrome border, deepest shadow (additive)
  #0a0e17  bg-base    — app shell background  ← CANONICAL (agent-def)
  #0d1220  bg-layer   — intermediate panel/content area background (additive)
  #121826  bg-surface — primary panel/card surface  ← NEUMORPHIC BASE (agent-def)
  #171e2e  surface-2  — raised within surface (inputs, nested cards)
  #1d2740  surface-3  — highly raised (dropdown menus, tooltips)
  #253352  surface-4  — active/hover highlight on raised elements
```

Shadow pair for neumorphism on `bg-surface` (#121826):
- Dark shadow: `#07090f` (−30% luminance step)
- Light shadow: `#1d273d` (+40% luminance step, blue-shifted)

These are not purely darker/lighter — the light shadow carries a blue hue shift to maintain the "cool deep space" feel rather than reading as warm or grey.

```
CYBER ACCENTS:

  #0084ff  mechanic-blue  — primary safe action, links, selection highlight
  #ffc800  construction   — hardware warnings, probability Medium, attention
  #e5534b  danger         — unrecoverable files, critical errors, destructive actions
  #3fb950  recovered      — successfully recovered file, High probability tier
  #58a6ff  info           — informational tooltips, softer blue for data labels
  #9b59ff  accent-alt     — secondary accent (used rarely: probability sparklines, audit log)
```

```
TEXT STACK:

  #f9f8f6  text-primary   — headlines, file names, primary labels (agent-def canonical)
  #c8cfd9  text-dim       — sub-headings, prominent metadata (supplementary, additive)
  #8a94a6  text-secondary — supporting labels, timestamps, placeholders (agent-def canonical)
  #4a5466  text-disabled  — explicitly disabled states only (agent-def canonical)
  #2a3248  text-inverse   — text on high-chroma surfaces (warning banners, coloured callouts)
```

Note: `--color-text-dim` (#c8cfd9) is a supplementary token that sits between primary and secondary. It is not in the agent-def but is additive and does not conflict. Component specs should default to `text-secondary` for secondary text; `text-dim` is available where a brighter mid-tone is needed (e.g. card subtitles, disk names in the list). TPM ruling pending on `--color-text-dim` inclusion.

### Accessibility Contrast Audit

All values measured against WCAG 2.1, targeting AA (4.5:1 normal text, 3:1 large/bold). All values on `--color-bg-surface` (#121826).

| Foreground | Token | Background | Ratio | WCAG AA Normal | WCAG AA Large |
|------------|-------|------------|-------|----------------|---------------|
| `#f9f8f6` | text-primary | `#121826` surface | 14.9:1 | PASS | PASS |
| `#c8cfd9` | text-dim | `#121826` surface | 10.0:1 | PASS | PASS |
| `#8a94a6` | text-secondary | `#121826` surface | 5.1:1 | PASS | PASS |
| `#4a5466` | text-disabled | `#121826` surface | 2.3:1 | FAIL (intentional — disabled) | FAIL (intentional) |
| `#0084ff` | accent-primary | `#121826` surface | 4.65:1 | PASS | PASS |
| `#ffffff` on `#0084ff` button | text-on-accent | `#0084ff` | 3.4:1 | FAIL small | PASS large |
| `#3fb950` | tier-high / success | `#121826` surface | 5.1:1 | PASS | PASS |
| `#ffc800` | warning | `#121826` surface | 9.8:1 | PASS | PASS |
| `#e5534b` | danger | `#121826` surface | 5.8:1 | PASS | PASS |
| `#ffc800` | focus ring | `#121826` surface | 9.8:1 | PASS | PASS |

**Critical constraint on filled primary buttons:** White text on `#0084ff` achieves 3.4:1 — this passes WCAG AA Large Text (3:1) but NOT normal text (4.5:1). **Rule:** All text inside a filled `--color-accent-primary` button must be set at minimum `14px / font-weight 600` (which qualifies as "large bold text" under WCAG and requires only 3:1). This is enforced at the component spec level.

**Focus ring choice — Construction Yellow:** The agent-def mandates `--color-warning` (Construction Yellow) for the focus ring. This is the superior choice: 9.8:1 contrast on Firmament dark surface vs 4.65:1 for blue. Yellow is also more immediately visible in peripheral vision, which is critical for focus indicators in a dense data table.

**Neumorphic surface contrast warning:** Neumorphism creates depth via shadow, not via border. Interactive element boundaries rely purely on perceived 3D depth. The `--color-warning` focus ring (2px outline, 2px offset) is **mandatory** on all interactive elements. Neumorphism must never be the sole cue for interactivity.

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
--shadow-outset: 4px 4px 10px var(--color-shadow-dark), -4px -4px 10px var(--color-shadow-light)
--shadow-inset:  inset 4px 4px 10px var(--color-shadow-dark), inset -4px -4px 10px var(--color-shadow-light)
```
These are the values the agent-def Design Token Format pins. GSAP button animations call `var(--shadow-inset)` and `var(--shadow-outset)` directly — do not hardcode the pixel values in JS. Shadow color values are fixed to the Firmament palette: dark `#07090f`, light `#1d273d`.

**Levelled variants (for components needing more or less depth than standard):**
```
--shadow-raised-sm   6px 6px 12px ...   — table row hover, nested cards
--shadow-raised-md   8px 8px 18px ...   — heavier card/panel raise
--shadow-raised-lg   12px 12px 28px ...  — modals, floating dropdowns
--shadow-inset-sm    inset 3px 3px 6px  — selected row, active tab
--shadow-inset-md    inset 5px 5px 12px — active input, pressed card
```

### Interaction State Mapping

```
Button:
  Default   → var(--shadow-outset)
  Hover     → var(--shadow-outset) + translate(-1px, -1px), scale(1.005) — via GSAP
  Active    → var(--shadow-inset), translate(0,0), scale(1.0) — via GSAP
  Focus     → var(--shadow-outset) + 2px solid outline var(--color-warning), offset 2px
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

```
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
```
--duration-instant:    0ms    (reduced-motion fallback)
--duration-fastest:    80ms   (micro-interactions: icon swap, badge colour change)
--duration-fast:       140ms  (button press state, row hover)
--duration-base:       240ms  (panel reveal, modal appear)
--duration-slow:       400ms  (page-level transitions, onboarding steps)
--duration-crawl:      800ms  (progress bar smooth-fill easing)
```

**Easing (CSS cubic-bezier equivalents — GSAP uses these same curves):**
```
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
2. **Colour:** Default icon colour is `--color-text-secondary` (#8a94a6). Interactive/labelled icons (with visible text label alongside) use `--color-text-dim` (#c8cfd9). Semantic icons (success/warning/danger) use their respective accent colour.
3. **Size: use 16px or 20px.** 24px only for empty states and large contextual icons.
4. **Never scale icons with CSS `transform`** — always use a new size. Fractional pixel rendering destroys the stroke precision.

### Specific Icon Assignments

```
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

**One exception:** The app titlebar / window chrome can use a very subtle Perlin-noise grain at 3% opacity (not visible, but prevents the large flat surface from looking "digital" or "flat-flat"). This is optional and only applied to `--color-bg-base` (#0a0e17) areas.

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
    rgba(0, 132, 255, 0.04) 50%,
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

```
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

## 11. Open Questions for TPM

```
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

```
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
