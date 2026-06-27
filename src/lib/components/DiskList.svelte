<script lang="ts">
import type { DiskInfo } from '$lib/types';
import { formatBytes } from '$lib/utils/format';
/**
 * DiskList — P1 placeholder
 *
 * Full implementation is deferred to P1-T03. This stub renders a selectable
 * list of disk cards (or an empty state) so that the design-system layout can
 * be validated without real disk data.
 */
import { Database, HardDrive, Lock, LockOpen, Usb } from 'lucide-svelte';
import Badge from './Badge.svelte';

const {
  disks = [],
  onselect,
}: {
  disks?: DiskInfo[];
  onselect?: (disk: DiskInfo) => void;
} = $props();

function handleSelect(disk: DiskInfo): void {
  onselect?.(disk);
}

function handleKeydown(e: KeyboardEvent, disk: DiskInfo): void {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    handleSelect(disk);
  }
}
</script>

<div class="disk-list" aria-label="Available disks">
  {#if disks.length === 0}
    <div class="disk-list__empty" role="status">
      <Database size={24} aria-hidden="true" class="disk-list__empty-icon" />
      <p class="disk-list__empty-text">No disks found. Check permissions.</p>
    </div>
  {:else}
    <ul class="disk-list__items" role="listbox" aria-label="Disk selection">
      {#each disks as disk (disk.id)}
        <li role="option" aria-selected="false">
          <button
            class="disk-list__item"
            type="button"
            aria-label="Select {disk.display_name}"
            onclick={() => handleSelect(disk)}
            onkeydown={(e) => handleKeydown(e, disk)}
          >
            <span class="disk-list__item-icon" aria-hidden="true">
              {#if disk.drive_type === 'HDD'}
                <HardDrive size={20} />
              {:else if disk.drive_type === 'USB'}
                <Usb size={20} />
              {:else}
                <Database size={20} />
              {/if}
            </span>

            <span class="disk-list__item-info">
              <span class="disk-list__item-name">{disk.display_name}</span>
              <span class="disk-list__item-meta">
                <span class="disk-list__item-size">{formatBytes(disk.size_bytes)}</span>
                <Badge variant="default">{disk.filesystem}</Badge>
              </span>
            </span>

            <span class="disk-list__item-lock" aria-label={disk.encrypted ? 'Encrypted' : 'Not encrypted'}>
              {#if disk.encrypted}
                <Lock size={16} />
              {:else}
                <LockOpen size={16} />
              {/if}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .disk-list {
    width: 100%;
  }

  /* ── Empty state ─────────────────────────────────────────── */
  .disk-list__empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-10);
    color: var(--color-text-secondary);
    text-align: center;
  }

  :global(.disk-list__empty-icon) {
    color: var(--color-text-disabled);
  }

  .disk-list__empty-text {
    font-family: var(--font-family);
    font-size: var(--font-size-md);
    color: var(--color-text-secondary);
    margin: 0;
  }

  /* ── List ────────────────────────────────────────────────── */
  .disk-list__items {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  /* ── Item button ─────────────────────────────────────────── */
  .disk-list__item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-surface);
    border: none;
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-outset);
    cursor: pointer;
    text-align: left;
  }

  .disk-list__item:hover {
    background: var(--color-bg-surface-2);
  }

  .disk-list__item:focus-visible {
    outline: var(--focus-ring-width) solid var(--focus-ring-color);
    outline-offset: var(--focus-ring-offset);
  }

  .disk-list__item-icon {
    display: inline-flex;
    color: var(--color-text-secondary);
    flex-shrink: 0;
  }

  .disk-list__item-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    flex: 1;
    min-width: 0;
  }

  .disk-list__item-name {
    font-family: var(--font-family);
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .disk-list__item-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .disk-list__item-size {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .disk-list__item-lock {
    display: inline-flex;
    color: var(--color-text-secondary);
    flex-shrink: 0;
  }
</style>
