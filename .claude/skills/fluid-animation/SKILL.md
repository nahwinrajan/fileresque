---
name: fluid-animation
description: >
  Cinematic, physically-grounded animation system covering every interaction
  layer — micro-feedback to full-screen hero expansions. Use whenever building
  or improving animated UI for web (React, Next.js, Vue, Svelte — Framer Motion,
  GSAP, CSS), mobile (Flutter, React Native, Swift, Kotlin), or desktop
  (Electron, Tauri, SwiftUI, WinUI 3). Trigger for any request involving motion
  design, transitions, spring physics, or "making the UI feel smooth/alive" —
  even if the user doesn't say "animation" explicitly. Provides timing tables,
  easing curves, spring values, stagger patterns, performance tiers, and
  reduced-motion strategy. Skip for developer tooling, data-dense dashboards, or
  productivity software where snappy minimal motion is the correct default.
---

# Fluid Animation: Universal Cinematic Motion System

You produce motion that feels physically real, effortless, and alive across every platform
and every interaction layer. The reference aesthetic is premium native design — iOS Human
Interface Guidelines' best moments, Google Material You's spring physics, macOS's
spring-loaded spatial model — applied with intent rather than defaults.

This is the anti-`transition: all 0.3s ease` approach. Every animation decision is
deliberate: what moves, how far, how fast, what curve, what triggers the return.

---

## Part 0 — Before You Write a Single Keyframe

Animation quality is set at the architecture stage, not the styling stage.

### The Four Questions

Answer these before touching any animation code:

**1. What is this motion communicating?**
- Confirmation ("your tap registered")
- Causality ("this action caused that change")
- Hierarchy ("this element is more important than that one")
- State ("the system is loading / complete / errored")
- Spatial model ("you are going deeper / coming back / switching peers")

If you cannot name one of the above, the animation is decoration. Decoration is the
last 5% of polish, never the first decision.

**2. What layer does this live on?**
See the Layer Stack below. The layer determines timing tier, compositing strategy, and
interrupt behaviour.

**3. What is the performance envelope?**
Know your lowest target device before writing a single line. See `references/performance.md`.

**4. Is the destination state real before the animation starts?**
The hero element must exist at the destination. Animating into a ghost state creates
reflows on completion. Plan layouts first.

---

## Part 1 — The Layer Stack

Every animation belongs to exactly one layer. Layers have fixed timing budgets and do
not overlap in feel.

```
┌──────────────────────────────────────────────────────────────┐
│  Layer 5 — HERO / SPATIAL          550–750ms   Cinematic     │
│  Full-screen expansions, route transitions, page reveals     │
├──────────────────────────────────────────────────────────────┤
│  Layer 4 — OVERLAY / MODAL         350–500ms   Contextual    │
│  Sheets, dialogs, drawers, popovers, tooltips                │
├──────────────────────────────────────────────────────────────┤
│  Layer 3 — CONTENT / LIST          220–320ms   Silk          │
│  Card reveals, list stagger, content swap, skeleton-to-real  │
├──────────────────────────────────────────────────────────────┤
│  Layer 2 — STATE / FEEDBACK        120–200ms   Instant       │
│  Toggle, checkbox, switch, select, chip, badge, loading dot  │
├──────────────────────────────────────────────────────────────┤
│  Layer 1 — MICRO / PRESS           60–120ms    Subliminal    │
│  Button press depth, hover lift, tap ripple, icon morph      │
└──────────────────────────────────────────────────────────────┘
```

**Key rule:** Layers never fight. A Layer 1 press animation on a button that is
simultaneously a Layer 5 hero source must suppress the press animation during the
hero transition. Never run two layers on the same element simultaneously.

---

## Reference Map

Read the relevant file(s) for the task at hand:

| Task | Read |
|---|---|
| Choosing timing / duration / easing curves | `references/timing-and-curves.md` |
| Implementing a specific layer (press, toggle, stagger, modal, hero) | `references/layers-implementation.md` |
| Multi-element choreography / timelines | `references/layers-implementation.md` (Part 5) |
| Platform-specific rules (Framer Motion, Flutter, SwiftUI, Electron) | `references/platform-systems.md` |
| GPU compositing, blur cost, performance tiers, device budget | `references/performance.md` |
| Reduced motion / `prefers-reduced-motion` | `references/performance.md` (Part 8) |
| Anti-patterns to avoid / pre-ship checklist | `references/anti-patterns-checklist.md` |

When in doubt, start with `references/timing-and-curves.md` — the timing table and
easing curve set answer most questions about feel before any code is written.
