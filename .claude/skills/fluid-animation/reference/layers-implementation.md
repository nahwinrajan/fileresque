# Layer-by-Layer Implementation

## Part 4 — Implementation by Layer

### Layer 1 — Micro / Press

Every interactive element must have a press state. No exceptions. A button that does
not respond to touch is a broken button.

**The press model:** scale down on press (creates physical depth illusion), spring back
on release with slight overshoot (confirms the action completed).

```css
/* Web — CSS-only press for non-complex buttons */
.btn {
  transition: transform var(--dur-press, 70ms) var(--ease-exit),
              box-shadow var(--dur-press, 70ms) var(--ease-exit);
}
.btn:active {
  transform: scale(0.96);
  box-shadow: 0 1px 4px rgba(0,0,0,0.1);
}

/* Hover lift (desktop only — use @media (hover: hover)) */
@media (hover: hover) {
  .btn:hover {
    transform: translateY(-2px) scale(1.01);
    box-shadow: 0 8px 24px rgba(0,0,0,0.12);
    transition: transform 100ms var(--ease-content),
                box-shadow 100ms var(--ease-content);
  }
}
```

```jsx
// React — Framer Motion press with spring return
import { motion } from 'framer-motion'

const PressButton = ({ children, onClick }) => (
  <motion.button
    whileTap={{ scale: 0.95 }}
    whileHover={{ scale: 1.02, y: -2 }}
    transition={{ type: 'spring', stiffness: 400, damping: 17 }}
    onClick={onClick}
  >
    {children}
  </motion.button>
)
```

```dart
// Flutter — GestureDetector + AnimationController press
class AnimatedButton extends StatefulWidget { /* ... */ }

class _AnimatedButtonState extends State<AnimatedButton>
    with SingleTickerProviderStateMixin {
  late AnimationController _controller;
  late Animation<double> _scale;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(vsync: this);
    _scale = Tween(begin: 1.0, end: 0.95).animate(
      CurvedAnimation(parent: _controller, curve: Curves.easeOut),
    );
  }

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTapDown: (_) => _controller.animateTo(1.0,
          duration: const Duration(milliseconds: 70)),
      onTapUp: (_) => _controller.animateBack(0.0,
          duration: const Duration(milliseconds: 130),
          curve: Curves.elasticOut),
      onTapCancel: () => _controller.animateBack(0.0,
          duration: const Duration(milliseconds: 130)),
      child: ScaleTransition(scale: _scale, child: widget.child),
    );
  }
}
```

```swift
// SwiftUI — press animation
struct AnimatedButton: View {
  @State private var isPressed = false

  var body: some View {
    Button(action: {}) {
      // content
    }
    .scaleEffect(isPressed ? 0.95 : 1.0)
    .animation(.spring(response: 0.25, dampingFraction: 0.7), value: isPressed)
    .simultaneousGesture(
      DragGesture(minimumDistance: 0)
        .onChanged { _ in isPressed = true }
        .onEnded { _ in isPressed = false }
    )
  }
}
```

#### Icon Morphs

Play → Pause, Menu → Close, Search → Back. Layer 1 micro-interactions requiring
SVG path morphing, not scale.

```jsx
// React — Framer Motion SVG path morph
const playPath = "M 8 5 L 8 19 L 19 12 Z"
const pausePath = "M 6 5 H 10 V 19 H 6 Z M 14 5 H 18 V 19 H 14 Z"

const IconMorph = ({ isPlaying }) => (
  <svg viewBox="0 0 24 24" width={24} height={24}>
    <motion.path
      d={isPlaying ? pausePath : playPath}
      animate={{ d: isPlaying ? pausePath : playPath }}
      transition={{ duration: 0.18, ease: [0.16, 1, 0.3, 1] }}
      fill="currentColor"
    />
  </svg>
)
```

```dart
// Flutter — AnimatedIcon for built-in pairs (play_pause, menu_close, etc.)
// For complex morphs: use Rive (preferred for production)
AnimatedIcon(
  icon: AnimatedIcons.play_pause,
  progress: _controller,
  color: Colors.white,
  size: 28,
)
```

---

### Layer 2 — State / Feedback

Every state change must be visible. Loading, success, error, disabled — each is
a distinct animated state, not just a colour swap.

#### Toggle / Switch

```jsx
// React — Framer Motion toggle
const Toggle = ({ isOn, onToggle }) => (
  <motion.div
    className="toggle-track"
    onClick={onToggle}
    animate={{ backgroundColor: isOn ? '#34C759' : '#E5E5EA' }}
    transition={{ duration: 0.18, ease: [0.4, 0, 0.2, 1] }}
  >
    <motion.div
      className="toggle-thumb"
      animate={{ x: isOn ? 20 : 2 }}
      transition={{ type: 'spring', stiffness: 320, damping: 28 }}
    />
  </motion.div>
)
```

#### Loading States — Three Tiers

```
Tier A: Skeleton screens (< 1.5s expected load)
  → Shimmer animation over placeholder shapes matching real content geometry

Tier B: Spinner / pulse (1.5s – 4s expected load)
  → Indeterminate spinner; do not use determinate unless you have real progress data

Tier C: Progress + context (> 4s expected load)
  → Determinate bar + message explaining what is happening
```

```css
/* Skeleton shimmer — GPU composited */
@keyframes shimmer {
  0%   { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

.skeleton {
  background: linear-gradient(
    90deg,
    var(--color-surface-muted) 25%,
    var(--color-surface-subtle) 50%,
    var(--color-surface-muted) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.8s ease-in-out infinite;
  /* background-position is cheap on modern GPUs — exception to the GPU rule */
}
```

#### Error State Shake

```jsx
// React — error shake using keyframes variant
const shakeVariants = {
  idle: { x: 0 },
  error: {
    x: [0, -8, 8, -6, 6, -3, 3, 0],
    transition: { duration: 0.5, times: [0, 0.1, 0.3, 0.4, 0.55, 0.65, 0.75, 1] },
  },
}

const InputField = ({ hasError }) => (
  <motion.input
    variants={shakeVariants}
    animate={hasError ? 'error' : 'idle'}
  />
)
```

---

### Layer 3 — Content / List

#### List Stagger

```
Rules:
- Stagger delay: 45–65ms between items
- Per-item duration: 240–280ms
- Direction: translateY(10px) → 0 + opacity 0 → 1
- Max staggered items: 8. Beyond that, animate the first 8 and skip
  the rest (they enter instantly). Users do not see what is below
  the fold anyway.
- Stagger completes before user can interact. Do not let taps register
  mid-entrance — add pointer-events: none for the stagger duration.
```

```jsx
// React — Framer Motion stagger
const containerVariants = {
  hidden: {},
  visible: {
    transition: {
      staggerChildren: 0.055,
      delayChildren: 0.08,
    },
  },
}

const itemVariants = {
  hidden: { opacity: 0, y: 10 },
  visible: {
    opacity: 1,
    y: 0,
    transition: { duration: 0.26, ease: [0.16, 1, 0.3, 1] },
  },
}

const AnimatedList = ({ items }) => (
  <motion.ul variants={containerVariants} initial="hidden" animate="visible">
    {items.map((item) => (
      <motion.li key={item.id} variants={itemVariants}>
        <ItemCard item={item} />
      </motion.li>
    ))}
  </motion.ul>
)
```

```dart
// Flutter stagger
class StaggeredList extends StatefulWidget { /* ... */ }

class _State extends State<StaggeredList> with SingleTickerProviderStateMixin {
  late final AnimationController _ctrl = AnimationController(
    vsync: this,
    duration: Duration(milliseconds: 300 + widget.items.length * 55),
  )..forward();

  @override
  Widget build(BuildContext context) {
    return Column(
      children: List.generate(
        min(widget.items.length, 8),
        (i) {
          final start = (0.055 * i).clamp(0.0, 0.9);
          final end = (start + 0.28).clamp(0.0, 1.0);
          final anim = CurvedAnimation(
            parent: _ctrl,
            curve: Interval(start, end, curve: const Cubic(0.16, 1, 0.3, 1)),
          );
          return FadeTransition(
            opacity: anim,
            child: SlideTransition(
              position: Tween(begin: const Offset(0, 0.08), end: Offset.zero)
                  .animate(anim),
              child: widget.items[i],
            ),
          );
        },
      ),
    );
  }
}
```

#### Skeleton → Real Content Crossfade

Never hard-cut from skeleton to content. Crossfade over 300–400ms.

```jsx
// React
const ContentLoader = ({ isLoaded, skeleton, content }) => (
  <AnimatePresence mode="wait">
    {!isLoaded ? (
      <motion.div key="skeleton"
        exit={{ opacity: 0, transition: { duration: 0.2 } }}>
        {skeleton}
      </motion.div>
    ) : (
      <motion.div key="content"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1, transition: { duration: 0.35, ease: [0.16,1,0.3,1] } }}>
        {content}
      </motion.div>
    )}
  </AnimatePresence>
)
```

---

### Layer 4 — Overlay / Modal

#### Bottom Sheet (Mobile)

The sheet should feel like a physical object with weight — it falls into place.

```jsx
// React — Framer Motion bottom sheet
const sheetVariants = {
  hidden: { y: '100%', opacity: 0.7 },
  visible: {
    y: 0,
    opacity: 1,
    transition: { type: 'spring', stiffness: 300, damping: 32, mass: 1 },
  },
  exit: {
    y: '100%',
    opacity: 0,
    transition: { duration: 0.26, ease: [0.4, 0, 1, 1] },
  },
}

const BottomSheet = ({ isOpen, children }) => (
  <AnimatePresence>
    {isOpen && (
      <>
        <motion.div
          className="backdrop"
          initial={{ opacity: 0, backdropFilter: 'blur(0px)' }}
          animate={{ opacity: 1, backdropFilter: 'blur(16px) brightness(0.7)' }}
          exit={{ opacity: 0, backdropFilter: 'blur(0px)' }}
          transition={{ duration: 0.32, ease: [0.16, 1, 0.3, 1] }}
        />
        <motion.div
          className="sheet"
          variants={sheetVariants}
          initial="hidden"
          animate="visible"
          exit="exit"
        >
          {children}
        </motion.div>
      </>
    )}
  </AnimatePresence>
)
```

```dart
// Flutter — showModalBottomSheet with custom animation
showModalBottomSheet(
  context: context,
  isScrollControlled: true,
  backgroundColor: Colors.transparent,
  transitionAnimationController: AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 420),
    reverseDuration: const Duration(milliseconds: 280),
  )..forward(),
  builder: (ctx) => DraggableScrollableSheet(
    initialChildSize: 0.5,
    minChildSize: 0.25,
    maxChildSize: 0.95,
    builder: (ctx, sc) => SheetContent(scrollController: sc),
  ),
);
```

#### Dialog / Modal (Desktop + Web)

```jsx
// React — scale in from 95%, not slide from off-screen (feels grounded)
const modalVariants = {
  hidden: { opacity: 0, scale: 0.95, y: 8 },
  visible: {
    opacity: 1, scale: 1, y: 0,
    transition: { duration: 0.28, ease: [0.22, 1, 0.36, 1] },
  },
  exit: {
    opacity: 0, scale: 0.97, y: 4,
    transition: { duration: 0.18, ease: [0.4, 0, 1, 1] },
  },
}
```

```swift
// SwiftUI — modal with custom transition
.sheet(isPresented: $isPresented) {
  ModalContent()
    .presentationDetents([.medium, .large])
    .presentationDragIndicator(.visible)
    .presentationCornerRadius(28)
    // iOS 17+ spring sheet animation is automatic
}
```

---

### Layer 5 — Hero / Spatial

The signature interaction. A small element reveals its full form. The screen reorganises
around it. Everything else acknowledges this expansion.

#### The Hero Philosophy

The element does not "grow." It *reveals* what was always there. The large-format photo
was behind the small circle the whole time; you are removing the mask.

#### Web / React Implementation

```jsx
// Framer Motion layoutId — the hero pair
const CardThumbnail = ({ item, onExpand }) => (
  <motion.div
    layoutId={`card-${item.id}`}
    className="card-thumb"
    onClick={() => onExpand(item)}
    style={{ borderRadius: 16, overflow: 'hidden', cursor: 'pointer' }}
    transition={{ type: 'spring', stiffness: 120, damping: 22 }}
  >
    <motion.img
      layoutId={`card-img-${item.id}`}
      src={item.image}
      style={{ width: '100%', height: '100%', objectFit: 'cover' }}
    />
    <motion.h3 layoutId={`card-title-${item.id}`}>{item.title}</motion.h3>
  </motion.div>
)

const CardExpanded = ({ item, onCollapse }) => (
  <motion.div
    layoutId={`card-${item.id}`}
    className="card-expanded"
    onClick={onCollapse}
    style={{ borderRadius: 24, overflow: 'hidden' }}
    transition={{ type: 'spring', stiffness: 120, damping: 22 }}
  >
    <motion.img
      layoutId={`card-img-${item.id}`}
      src={item.image}
      style={{ width: '100%', height: '65vh', objectFit: 'cover' }}
    />
    <motion.h2 layoutId={`card-title-${item.id}`}>{item.title}</motion.h2>
    {/* Secondary content fades in — not shared, so use initial/animate */}
    <motion.div
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.15, duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
    >
      <p>{item.description}</p>
    </motion.div>
  </motion.div>
)

// Backdrop blur that tracks the hero
const HeroBackdrop = ({ isExpanded }) => (
  <AnimatePresence>
    {isExpanded && (
      <motion.div
        className="hero-backdrop"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1, backdropFilter: 'blur(20px) brightness(0.65)' }}
        exit={{ opacity: 0, backdropFilter: 'blur(0px)' }}
        transition={{ duration: 0.45, ease: [0.16, 1, 0.3, 1] }}
        style={{ position: 'fixed', inset: 0 }}
      />
    )}
  </AnimatePresence>
)
```

#### Flutter Implementation

```dart
// Hero widget for shared-element transition
class ProfileCard extends StatelessWidget {
  final String heroTag;
  final String photoUrl;
  final String name;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: () => Navigator.of(context).push(
        cinematicRoute(page: ProfileDetail(heroTag: heroTag, photoUrl: photoUrl)),
      ),
      child: Hero(
        tag: heroTag,
        flightShuttleBuilder: _morphShuttle,
        child: ClipRRect(
          borderRadius: BorderRadius.circular(12),
          child: Image.network(photoUrl, fit: BoxFit.cover),
        ),
      ),
    );
  }

  Widget _morphShuttle(BuildContext ctx, Animation<double> animation,
      HeroFlightDirection direction, BuildContext from, BuildContext to) {
    return AnimatedBuilder(
      animation: animation,
      builder: (ctx, child) {
        final radius = Tween<double>(begin: 12.0, end: 24.0).evaluate(
          CurvedAnimation(parent: animation, curve: const Cubic(0.16, 1, 0.3, 1)),
        );
        return ClipRRect(
          borderRadius: BorderRadius.circular(radius),
          child: Image.network(photoUrl, fit: BoxFit.cover),
        );
      },
    );
  }
}

// Cinematic route builder
PageRoute cinematicRoute({required Widget page}) => PageRouteBuilder(
  pageBuilder: (ctx, animation, secondary) => page,
  transitionDuration: const Duration(milliseconds: 420),
  reverseTransitionDuration: const Duration(milliseconds: 340),
  transitionsBuilder: (ctx, animation, secondary, child) {
    final enter = CurvedAnimation(
      parent: animation,
      curve: const Cubic(0.32, 0.72, 0, 1),
    );
    return FadeTransition(
      opacity: animation,
      child: ScaleTransition(scale: enter, child: child),
    );
  },
);
```

#### SwiftUI Matched Geometry (iOS)

```swift
@Namespace private var heroNamespace

struct CardGrid: View {
  @State private var selectedItem: Item? = nil

  var body: some View {
    ZStack {
      LazyVGrid(columns: columns) {
        ForEach(items) { item in
          CardThumbnail(item: item, namespace: heroNamespace)
            .onTapGesture {
              withAnimation(.spring(response: 0.55, dampingFraction: 0.82)) {
                selectedItem = item
              }
            }
            .opacity(selectedItem?.id == item.id ? 0 : 1)
        }
      }

      if let item = selectedItem {
        CardDetail(item: item, namespace: heroNamespace) {
          withAnimation(.spring(response: 0.45, dampingFraction: 0.86)) {
            selectedItem = nil
          }
        }
        .zIndex(1)
      }
    }
  }
}

struct CardThumbnail: View {
  let item: Item
  var namespace: Namespace.ID

  var body: some View {
    Image(item.imageName)
      .resizable()
      .scaledToFill()
      .frame(width: 160, height: 200)
      .clipShape(RoundedRectangle(cornerRadius: 16))
      .matchedGeometryEffect(id: item.id, in: namespace)
  }
}
```

---

## Part 5 — Multi-Layer Choreography

The most sophisticated quality: while the hero expands, three other things happen at
once and feel like a single unified action — not chaos.

### The Rule

All animations in one interaction:
1. Share the same trigger moment (t = 0)
2. Use the same easing family
3. Have a natural duration hierarchy (hero longest, secondaries complete before or with it)
4. Secondary animations may have a small positive delay (80–150ms) to prevent first-frame
   clutter — not to stagger for aesthetics.

```
Trigger: user taps card
  t=0ms    hero expansion begins           (620ms, --ease-hero)
  t=0ms    backdrop blur begins            (peaks at t=372ms, 60% of hero)
  t=0ms    sibling cards begin shrinking   (380ms, --ease-exit-soft)
  t=100ms  detail content fades up         (380ms, --ease-content)
  t=620ms  all animations complete
```

```js
// GSAP — multi-layer timeline
import gsap from 'gsap'

function expandCard(cardEl, backdropEl, siblingsEl, detailEl) {
  const tl = gsap.timeline()

  tl
    .to(cardEl, {
      width: '100%', height: '70vh',
      borderRadius: 24,
      duration: 0.62, ease: 'power3.out',
    }, 0)

    .to(backdropEl, {
      opacity: 1,
      backdropFilter: 'blur(20px) brightness(0.65)',
      duration: 0.46, ease: 'power2.out',
    }, 0)

    .to(siblingsEl, {
      scale: 0.88, opacity: 0.5,
      duration: 0.38, ease: 'power2.inOut',
    }, 0)

    .fromTo(detailEl,
      { opacity: 0, y: 20 },
      { opacity: 1, y: 0, duration: 0.38, ease: 'power2.out' },
      0.1,  // 100ms offset
    )

  return tl
}

// Reverse with interrupt handling (user taps close mid-expansion)
function collapseCard(tl) {
  tl.reverse()  // GSAP reverses along the same path — no snap
}
```
