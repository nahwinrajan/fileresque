# Anti-Patterns & Pre-Ship Checklist

## Part 9 — Anti-Patterns

These undermine everything. Never do them.

**Animating layout properties.** Width, height, margin, top, left — all trigger layout
recalculation on every frame. Use transform instead. If the layout must change, use
the FLIP technique (First, Last, Invert, Play): read the before and after positions,
then animate transform from the delta. Framer Motion's `layout` prop does this automatically.

**Uniform timing across the interface.** An app where everything moves at 280ms feels
like a slideshow. Duration reflects semantic weight. A tooltip appears faster than a modal.
A button press is faster than a page transition.

**Interrupted animations that snap.** If a hero is mid-expansion and the user taps
elsewhere, it must reverse along the same path. Spring-based animations handle this
naturally. Duration-based animations need explicit interrupt handling or they will snap
to end.

**Blur on scrolling content.** Never apply `backdrop-filter` to an element scrolling
over blurred content on mobile. The GPU cost is prohibitive.

**Animating opacity and visibility separately.** `visibility: hidden` is not animatable.
`opacity: 0` is. Use opacity for fade-out, then set `display: none` or `visibility: hidden`
after the animation completes — never before.

**Phantom destination states.** The hero element must have a real layout position at its
destination before the transition starts. Animating into a state that does not yet exist
in the DOM causes reflow on completion and a jarring jump.

**Stagger beyond 8 items.** The last items in a long stagger feel abandoned. Cap at 8,
animate the first 8, let the rest appear instantly. The user's attention is not tracking
item 14 entering from below the fold.

**Hover animations on touch-only devices.** Use `@media (hover: hover)` to gate any
hover interaction. On mobile, hover states either fire on tap (confusing) or never fire
(wasted code).

**`will-change` left on permanently.** `will-change: transform` reserves a compositing
layer. On a page with 40 cards, all set to `will-change: transform`, you are reserving
40 compositing layers. Apply it just before animation starts, remove it when the animation
ends.

```js
// Correct will-change lifecycle
element.style.willChange = 'transform'
await animate(element)
element.style.willChange = 'auto'
```

---

## Part 10 — Checklist Before Shipping

For every animated interaction:

```
□ What is this animation communicating? (If no answer: remove it.)
□ Is it GPU-composited only? (transform + opacity. No layout properties.)
□ Does it have an interrupt behaviour? (What happens if user taps during it?)
□ Does it respect prefers-reduced-motion?
□ Has it been tested on the lowest tier target device?
□ Does frame time stay below 10ms at 60fps on that device?
□ Is the exit animation shorter than the entrance?
□ Does the animation complete before the user can interact with the result?
□ Is the destination state real in the DOM/layout tree before the animation starts?
□ If backdrop blur: is the background static during the blur?
□ If stagger: are there 8 or fewer animated items?
□ If hero: does the element exist at both source and destination?
```
