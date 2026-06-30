<script lang="ts">
import type { DeletedFileEntry } from '$lib/types';
import { formatBytes } from '$lib/utils/format';
import { FileText, HelpCircle, Image, Music, Video } from 'lucide-svelte';

const {
  files = [],
  onrowclick,
  selectedInode = null,
}: {
  files?: DeletedFileEntry[];
  onrowclick?: (file: DeletedFileEntry) => void;
  selectedInode?: number | null;
} = $props();

// ── Virtual list state ───────────────────────────────────────────────────────

const ROW_HEIGHT = 40;
const BUFFER_ROWS = 15;

let containerEl = $state<HTMLElement | null>(null);
let containerHeight = $state(400);
let scrollTop = $state(0);

$effect(() => {
  if (!containerEl) return;
  const ro = new ResizeObserver(([entry]) => {
    containerHeight = entry.contentRect.height;
  });
  ro.observe(containerEl);
  return () => ro.disconnect();
});

let visibleStart = $derived(Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER_ROWS));
let visibleEnd = $derived(
  Math.min(files.length, Math.ceil((scrollTop + containerHeight) / ROW_HEIGHT) + BUFFER_ROWS)
);
let visibleFiles = $derived(files.slice(visibleStart, visibleEnd));
let totalHeight = $derived(files.length * ROW_HEIGHT);
let offsetTop = $derived(visibleStart * ROW_HEIGHT);

// ── Helpers ──────────────────────────────────────────────────────────────────

function formatDate(secs: number | null): string {
  if (secs == null) return '—';
  return new Date(secs * 1000).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

function inferExt(name: string | null): string {
  if (!name) return '';
  const dot = name.lastIndexOf('.');
  return dot >= 0 ? name.slice(dot + 1).toLowerCase() : '';
}

type FileKind = 'image' | 'video' | 'audio' | 'doc' | 'unknown';

function fileKind(name: string | null): FileKind {
  const ext = inferExt(name);
  if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'heic', 'bmp', 'tiff'].includes(ext)) return 'image';
  if (['mp4', 'mov', 'mkv', 'avi', 'webm', 'm4v'].includes(ext)) return 'video';
  if (['mp3', 'aac', 'flac', 'wav', 'ogg', 'm4a'].includes(ext)) return 'audio';
  if (['pdf', 'doc', 'docx', 'txt', 'md', 'xls', 'xlsx', 'ppt', 'pptx'].includes(ext)) return 'doc';
  return 'unknown';
}

function displayName(file: DeletedFileEntry): string {
  if (file.name) return file.name;
  return `recovered_${file.inode_id}`;
}
</script>

<div class="file-table" aria-label="Deleted files">
  {#if files.length === 0}
    <div class="file-table__empty" role="status">
      <p class="file-table__empty-text">Scan a disk to see deleted files.</p>
    </div>
  {:else}
    <!-- Column headers -->
    <div class="file-table__header" role="row" aria-label="Column headers">
      <span class="col col--icon" aria-hidden="true"></span>
      <span class="col col--name">Name</span>
      <span class="col col--size">Size</span>
      <span class="col col--date">Deleted</span>
      <span class="col col--fs">FS</span>
    </div>

    <!-- Virtual scroll container -->
    <div
      class="file-table__scroll"
      role="grid"
      aria-rowcount={files.length}
      bind:this={containerEl}
      onscroll={(e) => {
        scrollTop = (e.currentTarget as HTMLElement).scrollTop;
      }}
    >
      <!-- Full-height spacer keeps scrollbar accurate -->
      <div class="file-table__spacer" style="height: {totalHeight}px;">
        <!-- Rendered rows shifted to scroll position -->
        <div class="file-table__rows" style="transform: translateY({offsetTop}px);">
          {#each visibleFiles as file, i (file.inode_id)}
            {@const kind = fileKind(file.name)}
            {@const rowIndex = visibleStart + i + 1}
            <button
              class="file-table__row"
              class:file-table__row--selected={file.inode_id === selectedInode}
              style="height: {ROW_HEIGHT}px;"
              type="button"
              role="row"
              aria-rowindex={rowIndex}
              aria-selected={file.inode_id === selectedInode}
              onclick={() => onrowclick?.(file)}
            >
              <span class="col col--icon" aria-hidden="true">
                {#if kind === 'image'}
                  <Image size={14} strokeWidth={1.5} />
                {:else if kind === 'video'}
                  <Video size={14} strokeWidth={1.5} />
                {:else if kind === 'audio'}
                  <Music size={14} strokeWidth={1.5} />
                {:else if kind === 'doc'}
                  <FileText size={14} strokeWidth={1.5} />
                {:else}
                  <HelpCircle size={14} strokeWidth={1.5} />
                {/if}
              </span>
              <span class="col col--name" title={file.name ?? undefined}>
                {displayName(file)}
              </span>
              <span class="col col--size">{formatBytes(file.size_bytes)}</span>
              <span class="col col--date">{formatDate(file.deleted_at)}</span>
              <span class="col col--fs">{file.filesystem}</span>
            </button>
          {/each}
        </div>
      </div>
    </div>

    <div class="file-table__footer" aria-live="polite">
      {files.length.toLocaleString()} file{files.length === 1 ? '' : 's'} found
    </div>
  {/if}
</div>

<style>
  .file-table {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    contain: layout;
  }

  /* ── Empty state ─────────────────────────────────────────── */
  .file-table__empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    padding: var(--space-12);
    color: var(--color-text-secondary);
  }

  .file-table__empty-text {
    font-family: var(--font-family);
    font-size: var(--font-size-md);
    color: var(--color-text-secondary);
    margin: 0;
  }

  /* ── Column layout ───────────────────────────────────────── */
  .file-table__header,
  .file-table__row {
    display: grid;
    grid-template-columns: 28px 1fr 80px 100px 64px;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-3);
  }

  .file-table__header {
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    height: 32px;
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  /* ── Scroll container ────────────────────────────────────── */
  .file-table__scroll {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    position: relative;
    /* Smooth scrolling on macOS */
    -webkit-overflow-scrolling: touch;
  }

  .file-table__spacer {
    position: relative;
    width: 100%;
  }

  .file-table__rows {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    will-change: transform;
  }

  /* ── Row ─────────────────────────────────────────────────── */
  .file-table__row {
    width: 100%;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    color: var(--color-text-dim);
    font-family: var(--font-family);
    font-size: var(--font-size-sm);
    border-bottom: 1px solid var(--color-border);
    contain: layout style;
  }

  .file-table__row:hover {
    background: var(--color-bg-layer);
  }

  .file-table__row--selected {
    background: var(--color-bg-surface-2);
    color: var(--color-text-primary);
  }

  .file-table__row:focus-visible {
    outline: 2px solid var(--color-accent-primary);
    outline-offset: -2px;
  }

  /* ── Columns ─────────────────────────────────────────────── */
  .col--icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-secondary);
  }

  .col--name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
  }

  .col--size,
  .col--date,
  .col--fs {
    font-size: var(--font-size-xs);
    color: var(--color-text-secondary);
    font-family: var(--font-mono);
    white-space: nowrap;
  }

  /* ── Footer ──────────────────────────────────────────────── */
  .file-table__footer {
    height: 28px;
    display: flex;
    align-items: center;
    padding: 0 var(--space-3);
    font-size: var(--font-size-xs);
    color: var(--color-text-secondary);
    border-top: 1px solid var(--color-border);
    flex-shrink: 0;
  }
</style>
