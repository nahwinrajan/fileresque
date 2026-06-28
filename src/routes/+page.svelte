<script lang="ts">
import type { DeletedFileEntry, DiskInfo } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { onDestroy } from 'svelte';
import { DiskList, FileTable, PermissionGate } from '$lib/components/index.js';
import Button from '$lib/components/Button.svelte';
import { ScanLine, X } from 'lucide-svelte';

// ── App-level state machine ──────────────────────────────────────────────────

type AppState = 'idle' | 'scanning' | 'complete' | 'error';

let appState = $state<AppState>('idle');
let selectedDisk = $state<DiskInfo | null>(null);
let files = $state<DeletedFileEntry[]>([]);
let filesFound = $state(0);
let scanError = $state<string | null>(null);
let scanDurationMs = $state<number | null>(null);

// ── Tauri event listeners ────────────────────────────────────────────────────

type UnlistenFn = () => void;
let unlisteners: UnlistenFn[] = [];

async function attachScanListeners(): Promise<void> {
  unlisteners = await Promise.all([
    listen<DeletedFileEntry>('scan:file_found', ({ payload }) => {
      files = [...files, payload];
    }),

    listen<{ files_found: number }>('scan:progress', ({ payload }) => {
      filesFound = payload.files_found;
    }),

    listen<{ total_found: number; duration_ms: number }>('scan:complete', ({ payload }) => {
      filesFound = payload.total_found;
      scanDurationMs = payload.duration_ms;
      appState = 'complete';
    }),

    listen<{ message: string; recoverable: boolean }>('scan:error', ({ payload }) => {
      scanError = payload.message;
      appState = 'error';
    }),
  ]);
}

function detachScanListeners(): void {
  for (const fn of unlisteners) fn();
  unlisteners = [];
}

onDestroy(detachScanListeners);

// ── Actions ──────────────────────────────────────────────────────────────────

function handleDiskSelect(disk: DiskInfo): void {
  selectedDisk = disk;
  files = [];
  filesFound = 0;
  scanError = null;
  scanDurationMs = null;
  appState = 'idle';
}

async function startScan(): Promise<void> {
  if (!selectedDisk || appState === 'scanning') return;
  files = [];
  filesFound = 0;
  scanError = null;
  scanDurationMs = null;
  appState = 'scanning';

  detachScanListeners();
  await attachScanListeners();

  try {
    await invoke('start_scan', { diskId: selectedDisk.id });
  } catch (err) {
    scanError = err instanceof Error ? err.message : String(err);
    appState = 'error';
    detachScanListeners();
  }
}

async function cancelScan(): Promise<void> {
  try {
    await invoke('cancel_scan');
  } catch {
    // cancel is best-effort
  }
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}
</script>

<PermissionGate />

<main class="app-shell">
  <div class="titlebar" data-tauri-drag-region aria-label="FileResque title bar">
    <span class="wordmark" aria-label="FileResque">
      File<strong>Resque</strong>
    </span>
  </div>

  <div class="content">
    <!-- ── Left panel: disk picker ──────────────────────────────── -->
    <aside class="panel panel--left" aria-label="Disk selection">
      <div class="panel__header">
        <span class="panel__title">Disks</span>
      </div>
      <div class="panel__body">
        <DiskList onselect={handleDiskSelect} />
      </div>

      {#if selectedDisk}
        <div class="panel__footer">
          {#if appState === 'scanning'}
            <Button variant="danger" onclick={cancelScan}>
              <X size={14} strokeWidth={1.5} />
              Cancel scan
            </Button>
          {:else}
            <Button variant="primary" onclick={startScan}>
              <ScanLine size={14} strokeWidth={1.5} />
              Scan {selectedDisk.display_name}
            </Button>
          {/if}
        </div>
      {/if}
    </aside>

    <!-- ── Right panel: results ─────────────────────────────────── -->
    <section class="panel panel--right" aria-label="Scan results">
      {#if appState === 'scanning'}
        <div class="scan-progress" aria-live="polite">
          <div class="scan-progress__bar" role="progressbar" aria-label="Scanning…">
            <div class="scan-progress__fill"></div>
          </div>
          <span class="scan-progress__label">
            Scanning… {filesFound > 0 ? `${filesFound.toLocaleString()} files found` : ''}
          </span>
        </div>
      {:else if appState === 'error'}
        <div class="scan-error" role="alert">
          <p class="scan-error__msg">Scan failed: {scanError}</p>
          <Button variant="secondary" onclick={startScan}>Retry</Button>
        </div>
      {:else if appState === 'complete' && scanDurationMs != null}
        <div class="scan-summary" aria-live="polite">
          <span class="scan-summary__text">
            {files.length.toLocaleString()} deleted file{files.length === 1 ? '' : 's'} found in
            {formatDuration(scanDurationMs)}
          </span>
        </div>
      {/if}

      <div class="file-table-wrapper">
        <FileTable {files} />
      </div>
    </section>
  </div>
</main>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background-color: var(--color-bg-base);
  }

  .titlebar {
    height: var(--titlebar-height);
    background-color: var(--color-bg-base);
    display: flex;
    align-items: center;
    /* 68px = native traffic-light button area on macOS (DECISION-010) */
    padding-left: calc(var(--space-8) + 68px);
    padding-right: var(--space-4);
    -webkit-app-region: drag;
    flex-shrink: 0;
    border-bottom: 1px solid var(--color-border);
  }

  .wordmark {
    font-family: var(--font-family);
    font-size: var(--font-size-md);
    color: var(--color-text-primary);
    letter-spacing: var(--tracking-tight);
  }

  .wordmark strong {
    color: var(--color-accent-primary);
    font-weight: var(--font-weight-semibold);
  }

  /* ── Two-panel layout ────────────────────────────────────── */
  .content {
    flex: 1;
    display: grid;
    grid-template-columns: 280px 1fr;
    overflow: hidden;
  }

  .panel {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel--left {
    border-right: 1px solid var(--color-border);
    background: var(--color-bg-base);
  }

  .panel--right {
    background: var(--color-bg-base);
  }

  .panel__header {
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .panel__title {
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--color-text-secondary);
  }

  .panel__body {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-3) var(--space-3);
  }

  .panel__footer {
    padding: var(--space-3);
    border-top: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  /* ── Scan progress banner ────────────────────────────────── */
  .scan-progress {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .scan-progress__bar {
    flex: 1;
    height: 4px;
    background: var(--color-bg-layer);
    border-radius: var(--radius-pill);
    overflow: hidden;
  }

  .scan-progress__fill {
    height: 100%;
    width: 40%;
    background: var(--color-accent-primary);
    border-radius: var(--radius-pill);
    animation: scan-indeterminate 1.4s ease-in-out infinite;
  }

  @keyframes scan-indeterminate {
    0% { transform: translateX(-100%); }
    60% { transform: translateX(250%); }
    100% { transform: translateX(250%); }
  }

  .scan-progress__label {
    font-size: var(--font-size-xs);
    color: var(--color-text-secondary);
    white-space: nowrap;
  }

  /* ── Scan summary banner ─────────────────────────────────── */
  .scan-summary {
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .scan-summary__text {
    font-size: var(--font-size-xs);
    color: var(--color-text-secondary);
  }

  /* ── Error banner ────────────────────────────────────────── */
  .scan-error {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    background: color-mix(in srgb, var(--color-danger) 8%, transparent);
  }

  .scan-error__msg {
    font-size: var(--font-size-xs);
    color: var(--color-danger);
    margin: 0;
    flex: 1;
  }

  /* ── File table ──────────────────────────────────────────── */
  .file-table-wrapper {
    flex: 1;
    overflow: hidden;
  }
</style>
