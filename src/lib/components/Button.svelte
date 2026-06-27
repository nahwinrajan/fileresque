<script lang="ts">
import { gsap } from 'gsap';
import { LoaderCircle } from 'lucide-svelte';
import type { Snippet } from 'svelte';

const {
  variant = 'primary',
  size = 'md',
  disabled = false,
  loading = false,
  type = 'button',
  onclick,
  children,
}: {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  loading?: boolean;
  type?: 'button' | 'submit' | 'reset';
  onclick?: (e: MouseEvent) => void;
  children?: Snippet;
} = $props();

let buttonEl: HTMLButtonElement;

// Checked once at module evaluation time — safe for SSR (typeof guard) and stable
// across the component lifetime. Reduced-motion users get instant state changes.
const prefersReducedMotion =
  typeof window !== 'undefined'
    ? window.matchMedia('(prefers-reduced-motion: reduce)').matches
    : false;

const isInteractive = $derived(!disabled && !loading);
const hasBoxShadow = $derived(variant !== 'ghost');

function handleMouseenter(): void {
  if (!isInteractive || prefersReducedMotion || !hasBoxShadow) return;
  gsap.to(buttonEl, { x: -1, y: -1, duration: 0.14, ease: 'power2.out' });
}

function handleMouseleave(): void {
  if (prefersReducedMotion || !hasBoxShadow) return;
  gsap.to(buttonEl, {
    x: 0,
    y: 0,
    boxShadow: 'var(--shadow-outset)',
    duration: 0.2,
    ease: 'power1.out',
  });
}

function handleMousedown(): void {
  if (!isInteractive || prefersReducedMotion || !hasBoxShadow) return;
  gsap.to(buttonEl, {
    boxShadow: 'var(--shadow-inset)',
    x: 0,
    y: 0,
    duration: 0.12,
    ease: 'power2.in',
  });
}

function handleMouseup(): void {
  if (!isInteractive || prefersReducedMotion || !hasBoxShadow) return;
  gsap.to(buttonEl, {
    boxShadow: 'var(--shadow-outset)',
    duration: 0.35,
    ease: 'elastic.out(1, 0.4)',
  });
}

function handleClick(e: MouseEvent): void {
  if (!isInteractive) return;
  onclick?.(e);
}
</script>

<button
  bind:this={buttonEl}
  {type}
  disabled={disabled || loading}
  aria-disabled={disabled || loading}
  aria-busy={loading}
  class="btn btn--{variant} btn--{size}"
  class:btn--loading={loading}
  onmouseenter={handleMouseenter}
  onmouseleave={handleMouseleave}
  onmousedown={handleMousedown}
  onmouseup={handleMouseup}
  onclick={handleClick}
>
  {#if loading}
    <span class="btn__spinner" aria-hidden="true">
      <LoaderCircle size={16} />
    </span>
    <span class="sr-only">Loading…</span>
  {:else}
    {@render children?.()}
  {/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    border: none;
    border-radius: var(--radius-md);
    font-family: var(--font-family);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
    box-shadow: var(--shadow-outset);
    position: relative;
    outline: none;
    white-space: nowrap;
    user-select: none;
    line-height: var(--line-height-none);
  }

  /* Focus ring — mandatory for keyboard navigation (design-brief §3) */
  .btn:focus-visible {
    outline: var(--focus-ring-width) solid var(--focus-ring-color);
    outline-offset: var(--focus-ring-offset);
  }

  /* ── Size variants ────────────────────────────────────────── */
  .btn--sm {
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-size-sm);
    min-height: 28px;
  }

  .btn--md {
    padding: var(--space-2) var(--space-4);
    font-size: var(--font-size-md);
    min-height: 36px;
  }

  .btn--lg {
    padding: var(--space-3) var(--space-6);
    font-size: var(--font-size-lg);
    min-height: 44px;
  }

  /* ── Colour variants ──────────────────────────────────────── */

  /* WCAG: text must be ≥14px/600 on accent — enforced by font-size-md + font-weight-semibold */
  .btn--primary {
    background: var(--color-accent-primary);
    color: var(--color-text-on-accent);
  }

  .btn--secondary {
    background: var(--color-bg-surface-2);
    color: var(--color-text-primary);
  }

  .btn--ghost {
    background: transparent;
    color: var(--color-text-secondary);
    box-shadow: none;
  }

  .btn--ghost:hover:not(:disabled) {
    background: var(--color-hover-overlay);
  }

  .btn--danger {
    background: var(--color-danger);
    color: var(--color-text-on-accent);
  }

  /* ── State: disabled ─────────────────────────────────────── */
  .btn:disabled,
  .btn[aria-disabled='true'] {
    box-shadow: var(--shadow-flat);
    opacity: 0.38;
    cursor: not-allowed;
  }

  /* Ghost disabled: no shadow to reset, only opacity */
  .btn--ghost:disabled,
  .btn--ghost[aria-disabled='true'] {
    box-shadow: none;
  }

  /* ── State: loading ──────────────────────────────────────── */
  .btn--loading {
    cursor: wait;
  }

  .btn__spinner {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    animation: btn-spin 1s linear infinite;
  }

  @keyframes btn-spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  /* ── Accessibility helper ────────────────────────────────── */
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
