<script lang="ts">
import { gsap } from 'gsap';
import type { Snippet } from 'svelte';

const {
  padding = 'md',
  selected = false,
  hoverable = false,
  children,
}: {
  padding?: 'none' | 'sm' | 'md' | 'lg';
  selected?: boolean;
  hoverable?: boolean;
  children?: Snippet;
} = $props();

let cardEl: HTMLDivElement;

const prefersReducedMotion =
  typeof window !== 'undefined'
    ? window.matchMedia('(prefers-reduced-motion: reduce)').matches
    : false;

// Register hover handlers imperatively via $effect to avoid the Svelte
// a11y_no_static_element_interactions warning — the mouse events are
// purely cosmetic (GSAP shadow lift), not functional interactions.
$effect(() => {
  if (!hoverable || !cardEl) return;

  function onEnter(): void {
    if (prefersReducedMotion) return;
    gsap.to(cardEl, { boxShadow: 'var(--shadow-raised-md)', duration: 0.2, ease: 'power2.out' });
  }

  function onLeave(): void {
    if (prefersReducedMotion) return;
    gsap.to(cardEl, { boxShadow: 'var(--shadow-outset)', duration: 0.2, ease: 'power2.out' });
  }

  cardEl.addEventListener('mouseenter', onEnter);
  cardEl.addEventListener('mouseleave', onLeave);

  return () => {
    cardEl.removeEventListener('mouseenter', onEnter);
    cardEl.removeEventListener('mouseleave', onLeave);
  };
});
</script>

<div
  bind:this={cardEl}
  class="card card--padding-{padding}"
  class:card--selected={selected}
  class:card--hoverable={hoverable}
>
  {#if selected}
    <div class="card__accent-bar" aria-hidden="true"></div>
  {/if}
  <div class="card__content">
    {@render children?.()}
  </div>
</div>

<style>
  .card {
    background: var(--color-bg-surface);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-outset);
    position: relative;
    overflow: hidden;
  }

  /* Selected: outset shadow retained + accent border + left bar */
  .card--selected {
    border: 1px solid var(--color-accent-primary);
  }

  .card--hoverable {
    cursor: pointer;
  }

  /* Left accent bar — 2px indicator for selected card (design-brief §5) */
  .card__accent-bar {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--color-accent-primary);
    border-radius: var(--radius-xs) 0 0 var(--radius-xs);
    z-index: var(--z-raised);
  }

  /* Padding variants */
  .card__content {
    height: 100%;
  }

  .card--padding-none .card__content {
    padding: 0;
  }

  .card--padding-sm .card__content {
    padding: var(--space-3);
  }

  .card--padding-md .card__content {
    padding: var(--space-4);
  }

  .card--padding-lg .card__content {
    padding: var(--space-6);
  }
</style>
