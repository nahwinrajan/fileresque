<script lang="ts">
import { gsap } from 'gsap';
import { X } from 'lucide-svelte';
import type { Snippet } from 'svelte';

const {
  open = false,
  title,
  onclose,
  children,
  footer,
}: {
  open?: boolean;
  title?: string;
  onclose?: () => void;
  children?: Snippet;
  footer?: Snippet;
} = $props();

// $state required: panelEl lives inside {#if open}, so bind:this assigns it
// dynamically. Without $state, $effect would not re-run when panelEl is set.
// biome-ignore lint/style/useConst: bind:this in the template reassigns panelEl at runtime; Biome's static analysis cannot see Svelte template assignments
let panelEl: HTMLDivElement | undefined = $state();

const prefersReducedMotion =
  typeof window !== 'undefined'
    ? window.matchMedia('(prefers-reduced-motion: reduce)').matches
    : false;

// Run entrance animation whenever the modal opens and the panel element exists.
$effect(() => {
  if (!open || !panelEl) return;

  if (prefersReducedMotion) {
    panelEl.style.opacity = '1';
    panelEl.style.transform = 'none';
  } else {
    gsap.fromTo(
      panelEl,
      { opacity: 0, y: 8 },
      { opacity: 1, y: 0, duration: 0.25, ease: 'power2.out' }
    );
  }

  // Move focus inside the panel; tabindex="-1" allows it to receive focus
  // programmatically without appearing in the tab order.
  panelEl.focus();
});

function handleKeydown(e: KeyboardEvent): void {
  // Guard: only respond when modal is actually open.
  if (!open || e.key !== 'Escape') return;
  onclose?.();
}

// Close only when the click lands on the backdrop itself, not inside the panel.
// This avoids needing a separate stopPropagation handler on the panel div.
function handleBackdropClick(e: MouseEvent): void {
  if (panelEl && e.target instanceof Node && !panelEl.contains(e.target)) {
    onclose?.();
  }
}

// Keyboard handler paired with onclick so the backdrop satisfies
// a11y_click_events_have_key_events. Escape is handled globally via
// svelte:window but Enter here mirrors the click-to-close behaviour.
function handleBackdropKeydown(e: KeyboardEvent): void {
  if (e.key === 'Enter') {
    onclose?.();
  }
}
</script>

<!--
  svelte:window must be at the top level — cannot be inside {#if}.
  The handler guards with `if (!open)` so it is a no-op when the modal is closed.
-->
<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div
    class="modal__backdrop"
    role="presentation"
    onclick={handleBackdropClick}
    onkeydown={handleBackdropKeydown}
  >
    <div
      bind:this={panelEl}
      class="modal__panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby={title ? 'modal-title' : undefined}
      tabindex="-1"
    >
      <div class="modal__header">
        {#if title}
          <h2 id="modal-title" class="modal__title">{title}</h2>
        {/if}
        <button
          class="modal__close"
          type="button"
          aria-label="Close modal"
          onclick={onclose}
        >
          <X size={16} aria-hidden="true" />
        </button>
      </div>

      <div class="modal__body">
        {@render children?.()}
      </div>

      {#if footer}
        <div class="modal__footer">
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  /* Backdrop — colour-mix avoids hardcoding the rgba value;
     --color-bg-void (#070912) at 85% opacity matches the spec. */
  .modal__backdrop {
    position: fixed;
    inset: 0;
    background: color-mix(in srgb, var(--color-bg-void) 85%, transparent);
    z-index: var(--z-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-4);
  }

  .modal__panel {
    background: var(--color-bg-surface);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-raised-lg);
    width: 100%;
    max-width: 560px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    /* Remove browser default focus outline — handled by design-system :focus-visible */
    outline: none;
  }

  .modal__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-5) var(--space-6);
    border-bottom: 1px solid var(--color-border-subtle);
    flex-shrink: 0;
  }

  .modal__title {
    font-family: var(--font-family);
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
    color: var(--color-text-primary);
    margin: 0;
    line-height: var(--line-height-tight);
  }

  .modal__close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--color-text-secondary);
    cursor: pointer;
    padding: var(--space-1);
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }

  .modal__close:hover {
    color: var(--color-text-primary);
    background: var(--color-hover-overlay);
  }

  .modal__body {
    padding: var(--space-6);
    overflow-y: auto;
    flex: 1;
    font-family: var(--font-family);
    font-size: var(--font-size-lg);
    color: var(--color-text-secondary);
    line-height: var(--line-height-relaxed);
  }

  .modal__footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-6);
    border-top: 1px solid var(--color-border-subtle);
    flex-shrink: 0;
  }
</style>
