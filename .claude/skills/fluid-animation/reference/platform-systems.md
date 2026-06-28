# Platform-Specific Systems

## Part 6 — Platform Rules

### Web / React

- Use Framer Motion `layout` + `layoutId` for hero transitions. It handles spatial
  interpolation automatically via the FLIP algorithm.
- Prefer `transform` and `opacity` only. No animating `width`, `height`, `margin`,
  `padding`, or `top/left` — these trigger layout recalculation.
- Exception: `layoutId` in Framer Motion animates size via FLIP (reads layout, then
  transforms to match). This is safe.
- For complex timelines (5+ elements moving together), use GSAP. Framer Motion's
  declarative model becomes unwieldy past 3 elements.
- Always apply `will-change: transform` during animation, remove after. Do not leave
  it on permanently — it reserves a compositing layer and costs VRAM.
- Always implement `useReducedMotion()` from Framer Motion.

```jsx
// Global reduced motion wrapper
import { useReducedMotion, MotionConfig } from 'framer-motion'

const AppMotionConfig = ({ children }) => {
  const shouldReduceMotion = useReducedMotion()

  return (
    <MotionConfig
      reducedMotion={shouldReduceMotion ? 'always' : 'never'}
      transition={shouldReduceMotion
        ? { duration: 0.01 }      // near-instant, not zero (zero can break layout)
        : undefined               // use component-level specs
      }
    >
      {children}
    </MotionConfig>
  )
}
```

---

### Flutter

- Use `Hero` widget for shared-element transitions — handles spatial interpolation natively.
- Supplement with `AnimationController` + `SpringSimulation` for physics quality.
- Use `RepaintBoundary` to isolate animated subtrees, especially those with blur effects.
- Never apply `BackdropFilter` on a rapidly-scrolling list. Apply only to static overlays.
- Pre-blur backgrounds using a blurred image asset when the background is static.
- On Impeller (iOS default, Android opt-in): keep animation math minimal on the UI thread.
- Respect `MediaQuery.of(context).disableAnimations`.

```dart
// Reduced motion check
final shouldReduce = MediaQuery.of(context).disableAnimations;
final duration = shouldReduce
    ? const Duration(milliseconds: 1)
    : const Duration(milliseconds: 420);
```

---

### SwiftUI / iOS

- `matchedGeometryEffect` is the native hero transition tool.
- Wrap in `withAnimation(.spring(...))` — never `withAnimation(.easeInOut)`.
- Use `.animation(.spring(...), value:)` not `.animation(nil)`.
- `.sensoryFeedback(.impact(.medium), trigger: value)` adds haptic confirmation —
  essential for Layer 1 micro-interactions on physical devices.
- Respect `@Environment(\.accessibilityReduceMotion)`.

```swift
@Environment(\.accessibilityReduceMotion) var reduceMotion

var springAnimation: Animation {
  reduceMotion
    ? .linear(duration: 0.01)
    : .spring(response: 0.55, dampingFraction: 0.82)
}
```

---

### macOS / Desktop

Hover is a primary interaction state, not secondary. Design hover lifts, previews,
and hover-triggered reveals as first-class animation states.

```swift
// SwiftUI macOS hover
struct DesktopCard: View {
  @State private var isHovered = false

  var body: some View {
    cardContent
      .scaleEffect(isHovered ? 1.015 : 1.0)
      .shadow(radius: isHovered ? 16 : 4, y: isHovered ? 8 : 2)
      .animation(.spring(response: 0.22, dampingFraction: 0.72), value: isHovered)
      .onHover { hovering in isHovered = hovering }
  }
}
```

```css
/* Web desktop hover — only fires when device has hover capability */
@media (hover: hover) {
  .card {
    transition: transform 120ms var(--ease-content),
                box-shadow 120ms var(--ease-content);
  }
  .card:hover {
    transform: translateY(-3px) scale(1.012);
    box-shadow: 0 12px 32px rgba(0,0,0,0.14);
  }
}
```

---

### Electron / Tauri (Cross-platform Desktop)

The animation layer is a web renderer (Chromium). Apply all Web/React rules.
Additionally:

- OS-level window animations (open, close, minimise) are system-controlled. Do not try
  to override them.
- Native menu bars and toolbars do not animate. Do not animate custom ones to "match" —
  the mismatch looks worse than the plain version.
- `prefers-reduced-motion` maps to OS accessibility settings on all platforms. Always
  honour it.
