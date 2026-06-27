<script lang="ts">
import type { Snippet } from 'svelte';

const {
  variant = 'default',
  size = 'sm',
  children,
}: {
  variant?:
    | 'default'
    | 'success'
    | 'warning'
    | 'danger'
    | 'info'
    | 'tier-high'
    | 'tier-medium'
    | 'tier-low'
    | 'tier-unknown';
  size?: 'sm' | 'md';
  children?: Snippet;
} = $props();
</script>

<span class="badge badge--{variant} badge--{size}">
  {@render children?.()}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    font-family: var(--font-family);
    font-weight: var(--font-weight-medium);
    letter-spacing: var(--tracking-wider);
    text-transform: uppercase;
    line-height: var(--line-height-none);
    white-space: nowrap;
  }

  /* ── Size variants ────────────────────────────────────────── */
  .badge--sm {
    font-size: var(--font-size-xs);
    padding: var(--space-0-5) var(--space-2);
  }

  .badge--md {
    font-size: var(--font-size-sm);
    padding: var(--space-1) var(--space-3);
  }

  /* ── Colour variants ──────────────────────────────────────── */

  .badge--default {
    background: var(--color-bg-surface-2);
    color: var(--color-text-secondary);
  }

  .badge--success {
    background: var(--color-tier-high-bg);
    color: var(--color-success);
  }

  .badge--warning {
    background: var(--color-tier-medium-bg);
    color: var(--color-warning);
  }

  .badge--danger {
    background: var(--color-tier-low-bg);
    color: var(--color-danger);
  }

  /* info: accent-info at 12% opacity — uses color-mix to avoid hardcoding the rgba value */
  .badge--info {
    background: color-mix(in srgb, var(--color-accent-info) 12%, transparent);
    color: var(--color-accent-info);
  }

  /* ── Probability tier variants ──────────────────────────── */

  .badge--tier-high {
    background: var(--color-tier-high-bg);
    color: var(--color-tier-high);
  }

  .badge--tier-medium {
    background: var(--color-tier-medium-bg);
    color: var(--color-tier-medium);
  }

  .badge--tier-low {
    background: var(--color-tier-low-bg);
    color: var(--color-tier-low);
  }

  .badge--tier-unknown {
    background: var(--color-tier-unknown-bg);
    color: var(--color-tier-unknown);
  }
</style>
