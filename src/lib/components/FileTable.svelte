<script lang="ts">
import type { DeletedFileEntry } from '$lib/types';
import { ScanLine } from 'lucide-svelte';

const {
  files = [],
}: {
  files?: DeletedFileEntry[];
} = $props();
</script>

<div class="file-table" aria-label="Deleted files">
  {#if files.length === 0}
    <div class="file-table__empty" role="status">
      <ScanLine size={24} aria-hidden="true" class="file-table__empty-icon" />
      <p class="file-table__empty-text">Scan a disk to see deleted files.</p>
    </div>
  {:else}
    <!--
      P2-T05: replace this placeholder with a virtualised table that can render
      10k+ rows without layout jank. The `files` prop is already wired up.
    -->
    <p class="file-table__placeholder-note">
      {files.length} file{files.length === 1 ? '' : 's'} found — full table coming in P2-T05.
    </p>
  {/if}
</div>

<style>
  .file-table {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  /* ── Empty state ─────────────────────────────────────────── */
  .file-table__empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-12);
    color: var(--color-text-secondary);
    text-align: center;
    flex: 1;
  }

  :global(.file-table__empty-icon) {
    color: var(--color-text-disabled);
  }

  .file-table__empty-text {
    font-family: var(--font-family);
    font-size: var(--font-size-md);
    color: var(--color-text-secondary);
    margin: 0;
  }

  /* ── P2 stub note ────────────────────────────────────────── */
  .file-table__placeholder-note {
    font-family: var(--font-family);
    font-size: var(--font-size-sm);
    color: var(--color-text-secondary);
    padding: var(--space-4);
    margin: 0;
  }
</style>
