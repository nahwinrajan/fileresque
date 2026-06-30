<script lang="ts">
import type { ProbabilityReport, ProbabilityTier } from '$lib/types';
import { formatBytes } from '$lib/utils/format';
import { AlertTriangle, Loader, ShieldCheck } from 'lucide-svelte';

const {
  report = null,
  loading = false,
  error = null,
  fileName = null,
}: {
  report?: ProbabilityReport | null;
  loading?: boolean;
  error?: string | null;
  fileName?: string | null;
} = $props();

// ── Tier presentation ─────────────────────────────────────────────────────────

type TierMeta = { badge: string; symbol: string; label: string };

const TIER_META: Record<ProbabilityTier, TierMeta> = {
  High: { badge: 'tier-high', symbol: '🟢', label: 'High' },
  Medium: { badge: 'tier-medium', symbol: '🟡', label: 'Medium' },
  Low: { badge: 'tier-low', symbol: '🔴', label: 'Low' },
};

const tier = $derived(report ? TIER_META[report.tier] : null);
const freePct = $derived(report ? Math.min(100, Math.max(0, report.free_blocks_pct)) : 0);
</script>

<section class="prob-panel" aria-label="Recovery probability">
  <div class="prob-panel__head">
    <span class="prob-panel__title">Recoverability</span>
    {#if fileName}
      <span class="prob-panel__file" title={fileName}>{fileName}</span>
    {/if}
  </div>

  <div class="prob-panel__body">
    {#if loading}
      <div class="prob-panel__state" role="status" aria-live="polite">
        <span class="prob-panel__spinner" aria-hidden="true">
          <Loader size={16} strokeWidth={1.5} />
        </span>
        <span>Assessing recoverability…</span>
      </div>
    {:else if error}
      <div class="prob-panel__state prob-panel__state--error" role="alert">
        <AlertTriangle size={16} strokeWidth={1.5} aria-hidden="true" />
        <span>{error}</span>
      </div>
    {:else if report && tier}
      <!-- Tier badge -->
      <div class="prob-panel__tier">
        <span class="badge badge--{tier.badge}" aria-label="Probability tier: {tier.label}">
          <span aria-hidden="true">{tier.symbol}</span>
          {tier.label}
        </span>
        <span class="prob-panel__est">
          ~{formatBytes(report.estimated_recoverable_bytes)} recoverable
        </span>
      </div>

      <!-- Free-block breakdown bar -->
      <div class="prob-panel__metric">
        <div class="prob-panel__metric-head">
          <span>Free blocks</span>
          <span class="prob-panel__metric-val">{Math.round(freePct)}%</span>
        </div>
        <div
          class="prob-panel__bar"
          role="progressbar"
          aria-valuenow={Math.round(freePct)}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-label="Free blocks"
        >
          <div class="prob-panel__bar-fill" style="width: {freePct}%;"></div>
        </div>
      </div>

      <!-- Flag chips -->
      <div class="prob-panel__flags">
        <span class="chip" class:chip--on={report.trim_active}>
          <ShieldCheck size={12} strokeWidth={1.5} aria-hidden="true" />
          TRIM {report.trim_active ? 'active' : 'off'}
        </span>
        <span class="chip" class:chip--on={report.blocks_zeroed}>
          Blocks {report.blocks_zeroed ? 'zeroed' : 'intact'}
        </span>
      </div>

      <!-- Warnings -->
      {#if report.warnings.length > 0}
        <ul class="prob-panel__warnings">
          {#each report.warnings as warning (warning)}
            <li>
              <AlertTriangle size={12} strokeWidth={1.5} aria-hidden="true" />
              {warning}
            </li>
          {/each}
        </ul>
      {/if}
    {:else}
      <div class="prob-panel__state prob-panel__state--empty" role="status">
        <span>Select a file to assess recoverability.</span>
      </div>
    {/if}
  </div>
</section>

<style>
  .prob-panel {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--color-border);
    background: var(--color-bg-layer);
    flex-shrink: 0;
    max-height: 40%;
    overflow-y: auto;
  }

  .prob-panel__head {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }

  .prob-panel__title {
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--color-text-secondary);
  }

  .prob-panel__file {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--color-text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .prob-panel__body {
    padding: var(--space-3) var(--space-4);
  }

  /* ── Loading / error / empty states ──────────────────────── */
  .prob-panel__state {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-sm);
    color: var(--color-text-secondary);
  }

  .prob-panel__state--error {
    color: var(--color-danger);
  }

  .prob-panel__spinner {
    display: inline-flex;
    animation: prob-spin 0.9s linear infinite;
  }

  @keyframes prob-spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* ── Tier row ────────────────────────────────────────────── */
  .prob-panel__tier {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    margin-bottom: var(--space-3);
  }

  .prob-panel__est {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--color-text-dim);
  }

  /* ── Metric bar ──────────────────────────────────────────── */
  .prob-panel__metric {
    margin-bottom: var(--space-3);
  }

  .prob-panel__metric-head {
    display: flex;
    justify-content: space-between;
    font-size: var(--font-size-xs);
    color: var(--color-text-secondary);
    margin-bottom: var(--space-1);
  }

  .prob-panel__metric-val {
    font-family: var(--font-mono);
    color: var(--color-text-dim);
  }

  .prob-panel__bar {
    height: 6px;
    background: var(--color-bg-surface);
    border-radius: var(--radius-pill);
    overflow: hidden;
  }

  .prob-panel__bar-fill {
    height: 100%;
    background: var(--color-accent-primary);
    border-radius: var(--radius-pill);
    /* Reveal fill on load; collapses to 0ms under prefers-reduced-motion. */
    transition: width var(--duration-base) var(--ease-out-cubic);
  }

  /* ── Flag chips ──────────────────────────────────────────── */
  .prob-panel__flags {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-0-5) var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-2xs);
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    background: var(--color-bg-surface-2);
    color: var(--color-text-secondary);
  }

  .chip--on {
    background: var(--color-tier-medium-bg);
    color: var(--color-warning);
  }

  /* ── Warnings ────────────────────────────────────────────── */
  .prob-panel__warnings {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .prob-panel__warnings li {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    font-size: var(--font-size-xs);
    color: var(--color-warning);
    line-height: var(--line-height-snug);
  }

  /* ── Badge tier variants (mirrors Badge.svelte for inline use) ─ */
  .badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    border-radius: var(--radius-sm);
    font-family: var(--font-family);
    font-weight: var(--font-weight-medium);
    letter-spacing: var(--tracking-wider);
    text-transform: uppercase;
    font-size: var(--font-size-xs);
    padding: var(--space-0-5) var(--space-2);
  }

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
</style>
