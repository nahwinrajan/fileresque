<script lang="ts">
import type { DiskInfo } from '$lib/types';
import { formatBytes } from '$lib/utils/format';
import { invoke } from '@tauri-apps/api/core';
import { AlertCircle, Cpu, Disc3, HardDrive, RefreshCw, Server, Usb } from 'lucide-svelte';
import Badge from './Badge.svelte';
import Button from './Button.svelte';
import Card from './Card.svelte';

interface Props {
  onselect?: (disk: DiskInfo) => void;
}

const { onselect }: Props = $props();

type DriveType = DiskInfo['drive_type'];
type LoadState = 'loading' | 'error' | 'empty' | 'ready';

let loadState = $state<LoadState>('loading');
let disks = $state<DiskInfo[]>([]);
let errorMessage = $state<string>('');
let selectedId = $state<string | null>(null);

async function loadDisks(): Promise<void> {
  loadState = 'loading';
  errorMessage = '';
  try {
    const result = await invoke<DiskInfo[]>('get_disks');
    disks = result;
    loadState = result.length === 0 ? 'empty' : 'ready';
  } catch (err) {
    errorMessage = err instanceof Error ? err.message : String(err);
    loadState = 'error';
  }
}

function selectDisk(disk: DiskInfo): void {
  selectedId = disk.id;
  onselect?.(disk);
}

function driveTypeLabel(driveType: DriveType): string {
  const labels: Record<DriveType, string> = {
    SSD: 'SSD',
    HDD: 'HDD',
    NVMe: 'NVMe',
    USB: 'USB',
    Virtual: 'Virtual',
    Unknown: 'Disk',
  };
  return labels[driveType] ?? 'Disk';
}

$effect(() => {
  loadDisks();
});
</script>

<div class="disk-list">
  {#if loadState === 'loading'}
    <div class="state-container">
      {#each [0, 1, 2] as _}
        <div class="skeleton-card" aria-hidden="true">
          <div class="skeleton-icon"></div>
          <div class="skeleton-lines">
            <div class="skeleton-line skeleton-line--wide"></div>
            <div class="skeleton-line skeleton-line--narrow"></div>
          </div>
        </div>
      {/each}
    </div>
  {:else if loadState === 'error'}
    <div class="state-container state-container--centered">
      <AlertCircle size={32} color="var(--color-danger)" strokeWidth={1.5} />
      <p class="state-message">
        {errorMessage.includes('Permission') || errorMessage.includes('permission')
          ? 'Full Disk Access is required. Grant access in System Settings → Privacy & Security.'
          : 'Could not enumerate disks. Check that the app has the necessary permissions.'}
      </p>
      <Button variant="secondary" onclick={loadDisks}>
        <RefreshCw size={14} strokeWidth={1.5} />
        Retry
      </Button>
    </div>
  {:else if loadState === 'empty'}
    <div class="state-container state-container--centered">
      <Disc3 size={32} color="var(--color-text-secondary)" strokeWidth={1.5} />
      <p class="state-message">No disks found.</p>
    </div>
  {:else}
    <ul class="disk-cards" role="listbox" aria-label="Available disks">
      {#each disks as disk (disk.id)}
        {@const isSelected = selectedId === disk.id}
        <li role="option" aria-selected={isSelected}>
          <Card hoverable selected={isSelected} padding="md">
            <button
              class="disk-card-btn"
              type="button"
              aria-label="Select {disk.display_name}"
              onclick={() => selectDisk(disk)}
            >
              <span class="disk-icon" aria-hidden="true">
                {#if disk.drive_type === 'USB'}
                  <Usb size={20} strokeWidth={1.5} color="var(--color-text-secondary)" />
                {:else if disk.drive_type === 'NVMe'}
                  <Cpu size={20} strokeWidth={1.5} color="var(--color-text-secondary)" />
                {:else if disk.drive_type === 'Virtual'}
                  <Server size={20} strokeWidth={1.5} color="var(--color-text-secondary)" />
                {:else}
                  <HardDrive size={20} strokeWidth={1.5} color="var(--color-text-secondary)" />
                {/if}
              </span>
              <span class="disk-info">
                <span class="disk-name">{disk.display_name}</span>
                <span class="disk-meta">
                  <span class="disk-size">{formatBytes(disk.size_bytes)}</span>
                  <span class="disk-badges">
                    <Badge variant="info">{driveTypeLabel(disk.drive_type)}</Badge>
                    {#if disk.filesystem !== 'Unknown'}
                      <Badge variant="default">{disk.filesystem}</Badge>
                    {/if}
                    {#if disk.encrypted}
                      <Badge variant="warning">Encrypted</Badge>
                    {/if}
                    {#if disk.trim_enabled}
                      <Badge variant="success">TRIM</Badge>
                    {/if}
                  </span>
                </span>
              </span>
            </button>
          </Card>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .disk-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .disk-cards {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .disk-cards li {
    display: contents;
  }

  .disk-card-btn {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    color: inherit;
    text-align: left;
  }

  .disk-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    background: var(--color-bg-layer);
    box-shadow: var(--shadow-inset);
  }

  .disk-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }

  .disk-name {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .disk-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .disk-size {
    font-size: var(--font-size-xs);
    color: var(--color-text-secondary);
    font-family: var(--font-mono);
  }

  .disk-badges {
    display: flex;
    gap: var(--space-1);
    flex-wrap: wrap;
  }

  .state-container {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-2);
  }

  .state-container--centered {
    align-items: center;
    padding: var(--space-8) var(--space-4);
    text-align: center;
  }

  .state-message {
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
    line-height: var(--line-height-relaxed);
    margin: 0;
    max-width: 320px;
  }

  .skeleton-card {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-outset);
  }

  .skeleton-icon {
    flex-shrink: 0;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    background: var(--color-bg-layer);
    box-shadow: var(--shadow-inset);
    animation: shimmer 1.5s ease-in-out infinite;
  }

  .skeleton-lines {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .skeleton-line {
    height: 10px;
    border-radius: var(--radius-sm);
    background: var(--color-bg-layer);
    animation: shimmer 1.5s ease-in-out infinite;
  }

  .skeleton-line--wide {
    width: 60%;
  }

  .skeleton-line--narrow {
    width: 35%;
  }

  @keyframes shimmer {
    0%,
    100% {
      opacity: 0.4;
    }
    50% {
      opacity: 0.7;
    }
  }
</style>
