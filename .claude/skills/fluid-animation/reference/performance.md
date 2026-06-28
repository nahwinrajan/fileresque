# Performance & Reduced Motion

## Part 7 — Performance System

### The GPU Compositing Rule

Only animate properties that the GPU composites independently:

```
✅ transform: translate, scale, rotate, skew
✅ opacity
✅ filter: blur (with caveats — see below)
✅ clip-path (in most modern browsers)

❌ width, height
❌ margin, padding
❌ top, left, right, bottom
❌ background-color (use opacity trick instead)
❌ border-radius on a large element being animated (can force repaint)
```

**The background-color trick:** Layer a solid-colour overlay with `opacity` animation
instead of animating `background-color`. The colour appears to change but only opacity
composites.

```jsx
// Animate apparent background-color via opacity overlay
<div style={{ position: 'relative' }}>
  <div className="base-bg" /> {/* static base colour */}
  <motion.div
    className="hover-bg"     /* target colour */
    style={{ position: 'absolute', inset: 0 }}
    animate={{ opacity: isHovered ? 1 : 0 }}
    transition={{ duration: 0.12 }}
  />
  <div className="content" style={{ position: 'relative' }}>
    {children}
  </div>
</div>
```

### Blur Performance

Backdrop blur (`backdrop-filter: blur()`) is expensive.

| Use case | Safe? | Notes |
|---|---|---|
| Static modal over static background | ✅ Yes | Background not redrawing |
| Modal over scrolling list | ⚠️ Caution | Limit sigma to ≤ 12px on mobile |
| Bottom sheet over animated content | ❌ No | Drop blur, use dark overlay |
| Full-screen hero backdrop | ✅ Yes | Background is static during hero |
| Blur on scroll (parallax blur) | ❌ No | Never |

```dart
// Flutter — blur sigma guidance
// max 20 for static, max 12 for dynamic-background contexts
// On low-end: skip BackdropFilter, use Container with dark opacity
final blurSigma = isLowEndDevice ? 0.0 : 18.0;
```

### Performance Tiers

```
Tier 1 — High-end (iPhone 15, Pixel 8, MacBook M-series, Surface Pro 9+)
  All effects. Full spring physics. Backdrop blur up to sigma 24. Hero expansion.
  Full stagger. Complex timelines.

Tier 2 — Mid-range (Pixel 6a, Galaxy A54, Intel i5 laptop, mid-tier Android)
  Full effects. Backdrop blur up to sigma 14. All spring physics.
  Reduce stagger to 6 items maximum.

Tier 3 — Low-end (Redmi 10A, Galaxy A15, budget Chromebooks, 4GB RAM desktops)
  Opacity + transform only. No backdrop blur (use dark overlay instead).
  No blur transitions. Reduce spring complexity — use tween with ease curves.
  Skip hero expansion spatial travel: crossfade the source/destination instead.
  Max 4 stagger items.
```

```dart
// Flutter — adaptive animation tier
enum DeviceTier { high, mid, low }

DeviceTier getDeviceTier(int totalRamMb) {
  if (totalRamMb >= 6144) return DeviceTier.high;
  if (totalRamMb >= 3072) return DeviceTier.mid;
  return DeviceTier.low;
}

// Usage
final tier = getDeviceTier(await DeviceInfo.totalRamMb());
final blurSigma = switch (tier) {
  DeviceTier.high => 20.0,
  DeviceTier.mid  => 12.0,
  DeviceTier.low  => 0.0,   // no blur, use overlay
};
```

```js
// Web — detect via performance heuristics
const getWebTier = () => {
  const mem = navigator.deviceMemory ?? 4  // GB, may be undefined
  const cores = navigator.hardwareConcurrency ?? 4
  if (mem >= 6 && cores >= 6) return 'high'
  if (mem >= 3 && cores >= 4) return 'mid'
  return 'low'
}
```

### The Animation Budget

```
Per-frame budget at 60fps = 16.67ms
Per-frame budget at 120fps = 8.33ms

Your animation code should consume no more than 6ms per frame at 60fps.
The remaining 10ms is for layout, paint, and JavaScript execution.

Test on your lowest tier target device.
If frame time exceeds 10ms during any animation: reduce, not remove.
If frame time exceeds 14ms: the animation ships broken. Fix it.
```

---

## Part 8 — Reduced Motion

Never eliminate motion entirely. Collapse it spatially.

```
Full motion → Reduced motion
  Spatial travel (translateX/Y/Z)  → Remove. Opacity only.
  Scale animation                  → Preserve at 50% magnitude.
  Blur transitions                 → Replace with opacity.
  Spring/bounce                    → Replace with linear opacity.
  Stagger                          → Collapse to simultaneous fade.
  Hero expansion                   → Crossfade. No positional travel.
  Page transition                  → Simple opacity crossfade.
```

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    /* Do NOT use 0ms — zero-duration transitions can break JS-driven
       animations that rely on transitionend events */
  }

  /* Restore meaningful opacity transitions */
  .animate-opacity {
    transition-duration: 200ms !important;
  }
}
```
