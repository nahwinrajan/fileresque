<script lang="ts">
import { gsap } from 'gsap';

const {
  value = 0,
  max = 100,
  label,
  showPercent = false,
  variant = 'default',
  animated = false,
}: {
  value?: number;
  max?: number;
  label?: string;
  showPercent?: boolean;
  variant?: 'default' | 'success' | 'danger';
  animated?: boolean;
} = $props();

let fillEl: HTMLDivElement;

const prefersReducedMotion =
  typeof window !== 'undefined'
    ? window.matchMedia('(prefers-reduced-motion: reduce)').matches
    : false;

/** Clamp value to [0, max] and express as a percentage (0–100). */
const pct = $derived(max > 0 ? Math.min(100, Math.max(0, (value / max) * 100)) : 0);
const pctDisplay = $derived(Math.round(pct));

// Tween reference — killed before each new animation to avoid stacking tweens.
// JUSTIFIED: null initial value is safe; $effect guard checks fillEl existence.
let activeTween: { kill: () => void } | null = null;

$effect(() => {
  // Read pct inside the effect so Svelte tracks it as a dependency.
  const targetWidth = `${pct}%`;

  if (!fillEl) return;

  activeTween?.kill();

  if (prefersReducedMotion) {
    fillEl.style.width = targetWidth;
  } else {
    activeTween = gsap.to(fillEl, {
      width: targetWidth,
      duration: 0.4,
      ease: 'power1.out',
    });
  }

  return () => {
    activeTween?.kill();
  };
});
</script>

<div
  class="progress"
  class:scanning={animated}
  role="progressbar"
  aria-valuenow={value}
  aria-valuemin={0}
  aria-valuemax={max}
  aria-label={label}
>
  {#if label || showPercent}
    <div class="progress__header">
      {#if label}
        <span class="progress__label">{label}</span>
      {/if}
      {#if showPercent}
        <span class="progress__percent" aria-hidden="true">{pctDisplay}%</span>
      {/if}
    </div>
  {/if}
  <div class="progress__track">
    <div
      bind:this={fillEl}
      class="progress__fill progress__fill--{variant}"
      style="width: 0%"
    ></div>
  </div>
</div>

<style>
  .progress {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .progress__header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-2);
  }

  .progress__label {
    font-family: var(--font-family);
    font-size: var(--font-size-sm);
    color: var(--color-text-secondary);
    line-height: var(--line-height-snug);
  }

  .progress__percent {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .progress__track {
    width: 100%;
    height: 6px;
    background: var(--color-bg-surface-2);
    border-radius: var(--radius-pill);
    box-shadow: var(--shadow-inset-sm);
    overflow: hidden;
  }

  .progress__fill {
    height: 100%;
    border-radius: var(--radius-pill);
    width: 0%;
  }

  .progress__fill--default {
    background: var(--color-accent-primary);
  }

  .progress__fill--success {
    background: var(--color-success);
  }

  .progress__fill--danger {
    background: var(--color-danger);
  }
</style>
