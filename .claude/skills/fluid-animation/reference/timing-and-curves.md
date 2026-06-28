# Timing & Easing Curves

## Part 2 — Timing Philosophy

These targets are calibrated for spatial distance, not aesthetics. The eye needs time
to track motion across a distance. Too fast → registers as a jump. Too slow → drags.

### Duration ↔ Distance Ratio

```
For hero and page transitions:
  Duration (ms) ≈ spatial distance (px) × 1.0 to 1.2

  A hero that moves 400px needs ~400–480ms to feel controlled.
  A hero that moves 800px needs ~800–960ms — but cap at 750ms and
  let the easing curve handle the deceleration.

For micro-interactions:
  Duration is fixed, not distance-based. The tap confirmation is always ~80ms.
  It does not matter if the button is 120px or 60px wide.
```

### Timing Reference Table

| Layer | Transition Type | Duration | Feel |
|---|---|---|---|
| L1 | Button/tap press depth | 60–80ms | Subliminal |
| L1 | Button/tap release | 100–130ms | Instant spring |
| L1 | Hover lift (desktop) | 80–120ms | Reactive |
| L1 | Icon morph (play→pause etc.) | 150–220ms | Snappy |
| L2 | Toggle / switch | 180–220ms | Satisfying snap |
| L2 | Checkbox / radio | 140–180ms | Crisp |
| L2 | Badge count change | 160–200ms | Springy pop |
| L2 | Loading state onset | 200ms | Smooth assumption |
| L3 | List item enter (per item) | 240–280ms | Silky |
| L3 | Card expand (in-place) | 280–340ms | Smooth reveal |
| L3 | Skeleton → real content | 300–400ms | Gentle materialise |
| L3 | Content swap / tab switch | 220–300ms | Snappy crossfade |
| L4 | Bottom sheet rise | 380–440ms | Natural |
| L4 | Modal appear | 340–400ms | Contextual |
| L4 | Popover / tooltip | 180–240ms | Fast |
| L4 | Drawer slide | 360–420ms | Weighted |
| L5 | Hero element expansion | 550–700ms | Cinematic |
| L5 | Page/route transition forward | 380–450ms | Carries meaning |
| L5 | Page/route transition back | 320–380ms | Lighter, faster |
| L5 | Full-screen reveal | 600–750ms | Breathes in |
| — | Background blur onset | 0ms start, peaks at 60% of sibling duration | Gradual |
| — | Any exit animation | 65–75% of entrance duration | Makes room |

**Exit animations are always shorter than entrances.** The user is waiting for the next
state. Never make them wait on the old one.

---

## Part 3 — Easing Curves

Never use CSS built-ins for anything meaningful. `ease-in-out` is a rounding error.

### The Core Curve Set

```css
:root {
  /* ── HERO / SPATIAL ── */
  /* Strong ease-out. Enters with momentum, decelerates into place. */
  --ease-hero:          cubic-bezier(0.16, 1, 0.3, 1);

  /* Page/route entrance. Similar to hero but slightly more momentum at start. */
  --ease-page:          cubic-bezier(0.32, 0.72, 0, 1);

  /* ── OVERLAYS / CONTENT ── */
  /* Sheet, modal, drawer. Confident entry, gentle landing. */
  --ease-overlay:       cubic-bezier(0.22, 1, 0.36, 1);

  /* Content reveal (cards, list items). Smooth but with purpose. */
  --ease-content:       cubic-bezier(0.25, 1, 0.5, 1);

  /* ── MICRO / SPRING ── */
  /* Small UI spring. Slight overshoot — buttons, chips, badges. */
  --ease-spring:        cubic-bezier(0.34, 1.56, 0.64, 1);

  /* Toggle/switch: confident snap, no overshoot needed. */
  --ease-snap:          cubic-bezier(0.4, 0, 0.2, 1);

  /* ── EXITS ── */
  /* Fast out. Makes room. Never linger. */
  --ease-exit:          cubic-bezier(0.4, 0, 1, 1);

  /* Gentle exit for content being replaced (not dismissed). */
  --ease-exit-soft:     cubic-bezier(0.36, 0, 0.66, 0);
}
```

### Flutter / Swift / Kotlin Spring Equivalents

```dart
// Flutter — SpringDescription values

// Hero expansion: critically damped, no bounce
const heroSpring = SpringDescription(mass: 1.0, stiffness: 120.0, damping: 22.0);

// Overlay / sheet: confident entry
const overlaySpring = SpringDescription(mass: 1.0, stiffness: 150.0, damping: 20.0);

// Card / content reveal
const contentSpring = SpringDescription(mass: 1.0, stiffness: 170.0, damping: 18.0);

// Small UI (button release, badge): subtle overshoot
const uiSpring = SpringDescription(mass: 1.0, stiffness: 200.0, damping: 15.0);

// Playful orb / icon: visible overshoot
const playfulSpring = SpringDescription(mass: 0.8, stiffness: 280.0, damping: 12.0);

// Toggle / switch: crisp, no bounce
const snapSpring = SpringDescription(mass: 1.0, stiffness: 320.0, damping: 28.0);
```

```swift
// SwiftUI — spring() modifier equivalents

// Hero
.animation(.spring(response: 0.55, dampingFraction: 0.82), value: isExpanded)

// Overlay
.animation(.spring(response: 0.42, dampingFraction: 0.86), value: isPresented)

// Small UI (slight bounce)
.animation(.spring(response: 0.28, dampingFraction: 0.68), value: isPressed)

// Toggle / snap
.animation(.spring(response: 0.22, dampingFraction: 0.95), value: isOn)
```

```kotlin
// Jetpack Compose — spring() and tween() specs

// Hero
val heroSpec = spring<Float>(dampingRatio = 0.82f, stiffness = Spring.StiffnessLow)

// Content
val contentSpec = spring<Float>(dampingRatio = 0.78f, stiffness = Spring.StiffnessMediumLow)

// Micro (slight bounce)
val uiSpec = spring<Float>(dampingRatio = 0.65f, stiffness = Spring.StiffnessMedium)

// Snap
val snapSpec = tween<Float>(durationMillis = 180, easing = FastOutSlowInEasing)
```
