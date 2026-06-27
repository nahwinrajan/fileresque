---
name: fluid-animation
version: 1.0.0
description: |
  Cinematic, fluid animation system. Produces highly dynamic, physically grounded
  motion that feels effortless and satisfying — not snappy or mechanical.
  Built from observing high-quality mobile micro-animation: hero expansions,
  blur-as-depth, spring choreography, and multi-layer transitions.
  
  Use when building: dating apps, lifestyle apps, premium mobile UIs,
  any product where motion is a core part of the brand experience.
  
  NOT for: productivity tools, dashboards, developer tooling — where Emil-style
  snappiness is correct. This skill is for interfaces where you want people to
  feel something when they tap.
  
  Compatible with: Flutter (AnimationController, spring physics, Hero widgets),
  React/Next.js (Framer Motion, GSAP, CSS), WebGL (Three.js for particle effects).
---

# Fluid Animation: Cinematic Mobile Motion

You produce motion that feels physically real, effortless, and alive. The reference
aesthetic is premium iOS design: generous timing, depth-of-field blur, spring physics,
hero expansions that carry context between states.

This is the opposite of "snappy." Snap belongs in Slack. Here, things breathe.

---

## The Core Philosophy

Emil Kowalski is right about many things — transform + opacity only, springs for
gesture interactions, no animate-all. But his timing guidance (150–300ms for UI) is
tuned for productivity software where animations are friction. 

For lifestyle and social apps, the animation *is* the product. A generous, beautiful
transition creates emotional response. It signals care. The 600ms hero expansion in
the reference video does not feel slow — it feels cinematic. The difference is in how
you fill that time: the motion must be continuously beautiful, never hanging or drifting.

### The Three Pillars

**1. Hero Continuity**
UI elements carry their identity from one state to the next. A circle avatar becomes
a full-screen photo. A card thumbnail becomes a full-screen profile. Nothing appears
from nowhere; everything has a traceable origin.

**2. Blur as Depth**
When a foreground element expands, the background blurs behind it. This mimics how
real-world focus works — when a camera focuses on a near object, the background
softens. Apply this both as:
- A backdrop-filter blur on the context layer as the hero element grows
- A progressive scale-down of background elements (creates parallax depth)

**3. Choreographed Multi-Layer Motion**
Multiple elements move simultaneously in a coordinated sequence. While the hero expands,
secondary elements fade, cluster, or rearrange. The whole screen is in motion but reads
as a single unified action, not chaos.

---

## Timing Philosophy

These are not hard rules. They are the feel you are calibrating toward.

| Transition type | Target duration | Feel |
|---|---|---|
| Button press feedback | 120–180ms | Instant, confirms the tap |
| List item stagger (per item) | 220–280ms | Silky, each item settles |
| Navigation page transition | 380–450ms | Contextual, carries meaning |
| Hero element expansion | 550–700ms | Cinematic, fills with intention |
| Full-screen reveal | 600–750ms | Breathes in, fully opens |
| Background blur onset | Starts at 0ms, peaks at 60% of hero duration | Gradual, follows the hero |

The key insight: the eye needs time to track something moving across most of the screen.
A hero that takes 500px from origin to destination needs at least 500ms to feel controlled.
Faster than that and it registers as a jump. Slower and it drags. Match your duration to
the spatial distance of the transition.

---

## Easing Curves

Never use CSS built-ins for anything meaningful. They are weak.

```
// The four you actually need:

// Hero expansion — strong ease-out, the expansion decelerates into place
--ease-hero:        cubic-bezier(0.16, 1, 0.3, 1)

// Page transition — enters with momentum, settles gently
--ease-page:        cubic-bezier(0.32, 0.72, 0, 1)

// Spring-like for small UI (buttons, chips, badges)
--ease-ui-spring:   cubic-bezier(0.34, 1.56, 0.64, 1)   // slight overshoot

// Exit / fade out — must be faster than entrance
--ease-exit:        cubic-bezier(0.4, 0, 1, 1)           // fast, makes room
```

For Flutter (spring physics):
```dart
// Hero expansion spring
SpringDescription heroSpring = SpringDescription(
  mass: 1.0,
  stiffness: 120.0,
  damping: 22.0,       // critically damped, no bounce
);

// Card lift spring (slight overshoot)
SpringDescription cardSpring = SpringDescription(
  mass: 1.0,
  stiffness: 180.0,
  damping: 14.0,       // underdamped, subtle bounce
);

// Orb/badge spring (snappy, playful overshoot)
SpringDescription orbSpring = SpringDescription(
  mass: 0.8,
  stiffness: 260.0,
  damping: 12.0,
);
```

---

## The Hero Expansion Pattern

The signature move. A small element — a circle avatar, a card thumbnail, an image chip —
expands to fill a significant portion of the screen. Everything else acknowledges this
expansion by blurring or dimming.

### How to think about it

The element does not "grow." It *reveals* what was always there. The large-format photo
was behind the small circle the whole time; you are just removing the mask.

Use `clip-path` or `borderRadius` animation combined with `transform: scale()` to execute
this. The clip-path approach is more flexible for non-standard shapes (circles morphing
to rounded rectangles). The scale + overflow hidden approach is simpler for rectangular
cards.

### CSS / Web implementation

```css
/* The element in its small/collapsed state */
.hero-source {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  overflow: hidden;
  
  /* This is the key — the full image already exists inside */
  img {
    width: 280px;        /* full destination width */
    height: 380px;       /* full destination height */
    transform: scale(0.157) translate(-50%, -50%);  /* squish to show face */
    transform-origin: center;
  }
}

/* Expanded state — driven by JS/Motion */
.hero-expanded {
  width: 100%;
  height: 70vh;
  border-radius: 24px;
  
  img {
    width: 100%;
    height: 100%;
    transform: scale(1) translate(0, 0);
    object-fit: cover;
    object-position: top center;  /* prioritize face */
  }
}
```

### Framer Motion implementation (React)

```jsx
import { motion, useSpring, useTransform } from 'framer-motion'

const HeroCard = ({ isExpanded, photo, name }) => {
  return (
    <motion.div
      layout
      layoutId={`card-${name}`}
      transition={{
        type: 'spring',
        stiffness: 120,
        damping: 22,
        duration: 0.65,
      }}
      style={{
        borderRadius: isExpanded ? 24 : 100,
        overflow: 'hidden',
        width: isExpanded ? '100%' : 44,
        height: isExpanded ? '70vh' : 44,
      }}
    >
      <motion.img
        src={photo}
        layoutId={`photo-${name}`}
        style={{ width: '100%', height: '100%', objectFit: 'cover' }}
      />
    </motion.div>
  )
}

// The background blur overlay — animates in sync with the hero
const BlurOverlay = ({ isExpanded }) => (
  <motion.div
    animate={{
      backdropFilter: isExpanded ? 'blur(20px) brightness(0.7)' : 'blur(0px) brightness(1)',
    }}
    transition={{
      duration: 0.5,
      ease: [0.16, 1, 0.3, 1],  // --ease-hero
    }}
    style={{ 
      position: 'fixed', 
      inset: 0, 
      pointerEvents: isExpanded ? 'auto' : 'none' 
    }}
  />
)
```

### Flutter implementation

```dart
// Use Flutter's built-in Hero widget for the shared element transition
// Combined with a custom AnimatedContainer for the clip morphing

class ProfileAvatar extends StatelessWidget {
  final String heroTag;
  final String photoUrl;
  final bool isExpanded;

  @override
  Widget build(BuildContext context) {
    return Hero(
      tag: heroTag,
      flightShuttleBuilder: (ctx, animation, direction, fromCtx, toCtx) {
        // Custom shuttle: morph clip-path from circle to rounded rect during flight
        return AnimatedBuilder(
          animation: animation,
          builder: (ctx, child) {
            final radius = Tween<double>(
              begin: isExpanded ? 22.0 : 200.0,  // end → start
              end: isExpanded ? 200.0 : 22.0,
            ).animate(CurvedAnimation(
              parent: animation,
              curve: Curves.fastOutSlowIn,
            )).value;
            
            return ClipRRect(
              borderRadius: BorderRadius.circular(radius),
              child: Image.network(photoUrl, fit: BoxFit.cover),
            );
          },
        );
      },
      child: ClipRRect(
        borderRadius: BorderRadius.circular(isExpanded ? 22.0 : 200.0),
        child: Image.network(photoUrl, fit: BoxFit.cover),
      ),
    );
  }
}

// Background blur that animates alongside the hero transition
// Using BackdropFilter with AnimatedOpacity
class BlurBackdrop extends StatelessWidget {
  final bool isActive;
  final Animation<double> animation;

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: animation,
      builder: (ctx, child) => BackdropFilter(
        filter: ImageFilter.blur(
          sigmaX: 20.0 * animation.value,
          sigmaY: 20.0 * animation.value,
        ),
        child: Container(
          color: Colors.black.withOpacity(0.5 * animation.value),
        ),
      ),
    );
  }
}
```

---

## Page Transitions

Every screen change should feel like physical navigation, not a cut.

### The Cinematic Page Pattern (observed from reference video)

On navigate forward:
1. Outgoing page: scales down to `0.94`, opacity fades to `0`, blur increases to `8px`
2. Incoming page: scales up from `0.96` to `1.0`, opacity rises from `0` to `1`
3. Both happen simultaneously over ~400ms
4. The nav bar header crossfades with the new screen title

On navigate back:
1. The reverse — outgoing page slides/scales back, incoming page scales up from `0.96`
2. Slightly faster than forward navigation (360ms vs 420ms) — backs feel lighter

```jsx
// Framer Motion page transition variants
const pageVariants = {
  initial: {
    opacity: 0,
    scale: 0.96,
    filter: 'blur(0px)',
  },
  animate: {
    opacity: 1,
    scale: 1,
    filter: 'blur(0px)',
    transition: {
      duration: 0.42,
      ease: [0.32, 0.72, 0, 1],
    },
  },
  exit: {
    opacity: 0,
    scale: 0.94,
    filter: 'blur(6px)',
    transition: {
      duration: 0.3,
      ease: [0.4, 0, 1, 1],   // fast exit
    },
  },
}
```

```dart
// Flutter: Custom PageRouteBuilder for cinematic transitions
PageRouteBuilder cinematicRoute({required Widget page}) {
  return PageRouteBuilder(
    pageBuilder: (ctx, animation, secondaryAnimation) => page,
    transitionDuration: const Duration(milliseconds: 420),
    reverseTransitionDuration: const Duration(milliseconds: 360),
    transitionsBuilder: (ctx, animation, secondary, child) {
      // Incoming: scale up from 0.96
      final scaleIn = Tween<double>(begin: 0.96, end: 1.0).animate(
        CurvedAnimation(parent: animation, curve: const Cubic(0.32, 0.72, 0, 1)),
      );
      // Outgoing: scale down to 0.92 as new page comes in
      final scaleOut = Tween<double>(begin: 1.0, end: 0.92).animate(
        CurvedAnimation(parent: secondary, curve: Curves.easeIn),
      );
      
      return FadeTransition(
        opacity: animation,
        child: ScaleTransition(
          scale: scaleIn,
          child: child,
        ),
      );
    },
  );
}
```

---

## List & Content Stagger

When multiple items enter a screen together, they should cascade in rather than
appearing all at once. The cascade creates a sense of the screen composing itself.

### Rules
- Stagger delay: 40–70ms between items (shorter than Emil's ranges — these need to feel fast, not theatrical)
- Per-item duration: 240–300ms
- Direction: translateY(12px) → translateY(0) + opacity 0 → 1
- Do not stagger more than 6 items. Beyond that, the last items feel abandoned.
- Always complete the stagger before the user can interact — do not let them tap
  an item mid-entrance.

```jsx
// Framer Motion stagger container
const container = {
  animate: {
    transition: {
      staggerChildren: 0.055,
      delayChildren: 0.1,  // small initial pause for page to settle
    },
  },
}

const listItem = {
  initial: { opacity: 0, y: 12 },
  animate: {
    opacity: 1,
    y: 0,
    transition: {
      duration: 0.28,
      ease: [0.16, 1, 0.3, 1],
    },
  },
}
```

```dart
// Flutter stagger using AnimationController + Interval
class StaggeredList extends StatefulWidget {
  final List<Widget> children;
  
  // ...

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        for (int i = 0; i < widget.children.length; i++)
          AnimatedBuilder(
            animation: _controller,
            builder: (ctx, child) {
              // Each item has its own interval window
              final start = 0.06 * i;
              final end = start + 0.28;
              final interval = CurvedAnimation(
                parent: _controller,
                curve: Interval(start.clamp(0, 1), end.clamp(0, 1),
                    curve: const Cubic(0.16, 1, 0.3, 1)),
              );
              return Opacity(
                opacity: interval.value,
                child: Transform.translate(
                  offset: Offset(0, 12 * (1 - interval.value)),
                  child: widget.children[i],
                ),
              );
            },
          ),
      ],
    );
  }
}
```

---

## The Blur Transition Pattern

Blur is not decoration — it conveys focus and depth. Use it intentionally.

### When to blur
- Background blurs when a modal, hero element, or overlay takes focus
- Exiting pages blur slightly as they recede (cinematic depth)
- Active filter states (e.g., "loading a new batch") blur the content to signal
  that the content is not yet ready for interaction

### What not to do
- Never blur text the user needs to read. Blur backgrounds, not foregrounds.
- Never apply CPU-expensive blur mid-gesture on low-end hardware. Trigger blur
  animations on gesture end, not during drag.
- Limit blur to one layer at a time. Layered blurs destroy performance.

### Performance guidance for blur
```dart
// Flutter: apply blur only when needed, cache when static
// Never use BackdropFilter on a rapidly-scrolling list
// Preferred: use a pre-blurred image asset for static backgrounds
// For dynamic blur: limit sigmaX/Y to max 24.0 on target devices

// Check performance first:
// Run on Redmi 10A equivalent. If it drops below 50fps during blur onset,
// reduce sigma or substitute with a semi-transparent dark overlay.
```

---

## Simultaneous Multi-Layer Choreography

The reference video's most sophisticated quality: while the hero expands, three other
things happen at once — the background blurs, the remaining avatars cluster into a strip,
and the profile details fade up from below. These feel like one action because they all
share the same easing curve and start from the same trigger point.

### The choreography rule
All animations in a single interaction should:
1. Start from the same trigger moment
2. Use the same base easing family (all spring, or all ease-hero)
3. Have a natural duration hierarchy: the hero is longest, secondaries complete before or with it

```
Trigger: user taps avatar
  t=0ms    hero expansion begins (600ms total)
  t=0ms    background blur begins (peaks at t=360ms, matches 60% hero completion)
  t=0ms    other avatars begin clustering (400ms, completes before hero)
  t=100ms  profile details begin fading up (400ms, completes with hero)
  t=600ms  hero settled, all animations complete
```

The small delay on profile details (100ms) prevents visual clutter at the first frame
of the transition.

---

## Platform-Specific Notes

### Flutter
- Use `Hero` widget for shared-element transitions — it handles the spatial interpolation
- Supplement with `AnimationController` + `SpringSimulation` for physics quality
- Use `RepaintBoundary` to isolate animated subtrees, especially for blur effects
- On Redmi 10A class devices: skip blur animations, substitute with opacity + dark overlay
- Impeller (iOS default, Android opt-in): all `CustomPainter` + `Canvas` operations
  run on the raster thread — keep animation math on the UI thread minimal
- Use `TickerProviderStateMixin` for multi-animation coordination on a single widget

### React / Next.js
- Framer Motion `layout` and `layoutId` handle hero transitions automatically
- Use `useReducedMotion()` — reduce but don't eliminate. Replace motion with opacity.
- `backdrop-filter: blur()` is expensive. Test on a mid-range Android in Chrome.
  Fallback: `rgba()` overlay at 70% opacity if blur is unavailable or slow.
- GSAP is the better choice for complex timeline choreography (multi-element sequences)
  where Framer Motion's declarative model becomes unwieldy.

### GSAP (for complex web choreography)
```js
// Multi-layer choreography using GSAP timeline
const tl = gsap.timeline({ defaults: { ease: 'power3.out' } })

tl
  // Hero expands
  .to('.hero-element', { 
    width: '100%', height: '70vh', borderRadius: 24,
    duration: 0.65, ease: 'power2.out' 
  }, 0)
  
  // Background blurs simultaneously
  .to('.backdrop', {
    backdropFilter: 'blur(20px) brightness(0.65)',
    duration: 0.5,
  }, 0)
  
  // Secondary avatars cluster (completes before hero)
  .to('.avatar-cluster', {
    x: 'var(--cluster-target-x)', 
    scale: 0.72, opacity: 0.9,
    duration: 0.4, ease: 'power3.inOut',
  }, 0)
  
  // Profile details fade up (slight delay)
  .fromTo('.profile-details', 
    { opacity: 0, y: 24 },
    { opacity: 1, y: 0, duration: 0.4, ease: 'power2.out' },
    0.1   // 100ms after trigger
  )
```

---

## Reduced Motion

Some users are sensitive to motion. The reduction strategy here differs from Emil's
approach: we do not eliminate animations, we collapse them spatially.

```css
@media (prefers-reduced-motion: reduce) {
  /* Hero expansion: keep scale, remove blur and spatial travel */
  .hero-element {
    transition: opacity 0.3s ease, width 0.3s ease, height 0.3s ease;
    /* Remove transform: translate, remove filter: blur */
  }
  
  /* Blur effects: replace with opacity */
  .blur-backdrop {
    backdrop-filter: none !important;
    background: rgba(0, 0, 0, 0.6) !important;
    transition: opacity 0.3s ease;
  }
  
  /* Stagger: collapse to simultaneous fade */
  .stagger-item {
    animation-delay: 0ms !important;
    transition: opacity 0.2s ease !important;
    transform: none !important;
  }
}
```

---

## Performance Constraints Reference

Before shipping any animation:

| Device class | Animation budget | Blur allowed? | Hero allowed? |
|---|---|---|---|
| High-end (iPhone 15, Pixel 8) | Full spec | Yes, up to sigma 24 | Yes, full spring |
| Mid-range (Pixel 6a, Galaxy A54) | Full spec | Yes, up to sigma 16 | Yes |
| Low-end (Redmi 10A, Galaxy A15) | Reduced | Avoid backdrop-filter | Scale + opacity only, no spatial travel |

Test on the lowest target device before declaring an animation production-ready.
The reference device for Senja is Redmi 10A. If it drops below 50fps on that device,
the animation needs adjustment — not removal.

```dart
// Flutter: detect low-end device at runtime
// Use DeviceInfoPlugin + heuristics based on total RAM
// < 3GB RAM → skip blur, use simplified transitions
final deviceMemory = await _deviceInfo.totalRam();  // MB
final isLowEndDevice = deviceMemory < 3072;
```

---

## What Not to Do

These patterns feel wrong and undo everything:

**Don't interrupt natural continuity.** If a hero is mid-expansion and the user taps
somewhere else, the hero should reverse along the same path it took, not snap.
Spring-based animations handle this naturally; duration-based ones need explicit interrupt
handling.

**Don't animate things that already feel right.** If the gesture itself is satisfying
(e.g., a native scroll with momentum), don't layer animation on top of it. Augment
what is weak, leave what is strong alone.

**Don't use uniform timing for everything.** An app where everything moves at 400ms
feels like a slideshow. Vary the duration to reflect the spatial and semantic weight
of each transition.

**Don't create phantom destination states.** The hero element must exist at the
destination before the transition starts. Plan the layout to accommodate this.
Animating into a state that isn't real creates jarring reflows on completion.

**Don't let blur outlast the context.** Background blur should dissolve before or
exactly when the hero completes. A blur lingering after the transition finished
looks like a bug, not polish.