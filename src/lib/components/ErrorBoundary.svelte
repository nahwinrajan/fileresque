<script lang="ts">
import { RotateCcw, TriangleAlert } from 'lucide-svelte';
import type { Snippet } from 'svelte';

const {
  children,
  label = 'this view',
}: {
  children: Snippet;
  /** Names the area that failed, e.g. "the disk list". Used in the message. */
  label?: string;
} = $props();
</script>

<!--
  Render-fault boundary (P5-T03). A crash inside one panel shows a recoverable
  fallback here instead of blanking the whole window. The error detail is kept
  out of the UI on purpose — it goes to the console for developers, never to the
  user, matching the no-raw-errors rule on the Rust side.
-->
<svelte:boundary onerror={(error) => console.error(`[ErrorBoundary:${label}]`, error)}>
  {@render children()}

  {#snippet failed(_error, reset)}
    <div class="boundary" role="alert">
      <TriangleAlert size={20} strokeWidth={1.5} aria-hidden="true" />
      <p class="boundary__title">Something went wrong in {label}.</p>
      <p class="boundary__hint">
        Your disks and files were not changed. You can retry without restarting the app.
      </p>
      <button type="button" class="boundary__retry" onclick={reset}>
        <RotateCcw size={14} strokeWidth={1.5} aria-hidden="true" />
        Try again
      </button>
    </div>
  {/snippet}
</svelte:boundary>

<style>
  .boundary {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    height: 100%;
    padding: var(--space-6);
    text-align: center;
    color: var(--color-text-secondary);
  }

  .boundary :global(svg) {
    color: var(--color-warning);
  }

  .boundary__title {
    margin: 0;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-primary);
  }

  .boundary__hint {
    margin: 0;
    max-width: 36ch;
    font-size: var(--font-size-xs);
    line-height: var(--line-height-relaxed);
  }

  .boundary__retry {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
    padding: var(--space-2) var(--space-3);
    font-size: var(--font-size-xs);
    color: var(--color-text-primary);
    background: var(--color-bg-layer);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    cursor: pointer;
  }

  .boundary__retry:hover {
    border-color: var(--color-accent-primary);
  }
</style>
