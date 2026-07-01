<script lang="ts">
import Button from '$lib/components/Button.svelte';
import {
  DiskList,
  ErrorBoundary,
  FileTable,
  PermissionGate,
  ProbabilityPanel,
  RecoveryModal,
} from '$lib/components/index.js';
import type { DeletedFileEntry, DiskDisconnectedEvent, DiskInfo } from '$lib/types';
import type { ProbabilityReport } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { HardDriveDownload, ScanLine, X } from 'lucide-svelte';
import { onDestroy } from 'svelte';

// ── App-level state machine ──────────────────────────────────────────────────

type AppState = 'idle' | 'scanning' | 'complete' | 'error';

let appState = $state<AppState>('idle');
let selectedDisk = $state<DiskInfo | null>(null);
let files = $state<DeletedFileEntry[]>([]);
let filesFound = $state(0);
let scanError = $state<string | null>(null);
let scanDurationMs = $state<number | null>(null);

// ── Probability assessment state (P3-T02) ────────────────────────────────────

let selectedFile = $state<DeletedFileEntry | null>(null);
let probReport = $state<ProbabilityReport | null>(null);
let probLoading = $state(false);
let probError = $state<string | null>(null);
/** Monotonic token: ignore stale assessments when rows are clicked quickly. */
let probRequestId = 0;

// ── Recovery flow state (P4-T03) ──────────────────────────────────────────────

let recoverOpen = $state(false);
let recoverDest = $state<string | null>(null);

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

    // Source disk pulled mid-scan (P5-T03): surface the friendly reason and
    // stop, so the user is never left staring at a frozen progress bar.
    listen<DiskDisconnectedEvent>('disk:disconnected', ({ payload }) => {
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

function resetProbability(): void {
  selectedFile = null;
  probReport = null;
  probError = null;
  probLoading = false;
  probRequestId += 1;
}

function handleDiskSelect(disk: DiskInfo): void {
  selectedDisk = disk;
  files = [];
  filesFound = 0;
  scanError = null;
  scanDurationMs = null;
  appState = 'idle';
  resetProbability();
}

async function handleRowClick(file: DeletedFileEntry): Promise<void> {
  if (!selectedDisk) return;
  selectedFile = file;
  probReport = null;
  probError = null;
  probLoading = true;
  const requestId = ++probRequestId;

  try {
    const report = await invoke<ProbabilityReport>('check_probability', {
      entry: file,
      disk: selectedDisk,
    });
    // Drop the result if a newer request superseded this one.
    if (requestId !== probRequestId) return;
    probReport = report;
  } catch (err) {
    if (requestId !== probRequestId) return;
    probError = err instanceof Error ? err.message : String(err);
  } finally {
    if (requestId === probRequestId) probLoading = false;
  }
}

async function handleRecoverClick(): Promise<void> {
  if (!selectedFile || !selectedDisk) return;
  // Native folder picker first; Ok(None) (user cancelled) → do nothing.
  const dest = await invoke<string | null>('pick_destination_folder');
  if (!dest) return;
  recoverDest = dest;
  recoverOpen = true;
}

function closeRecovery(): void {
  recoverOpen = false;
}

async function startScan(): Promise<void> {
  if (!selectedDisk || appState === 'scanning') return;
  files = [];
  filesFound = 0;
  scanError = null;
  scanDurationMs = null;
  appState = 'scanning';
  resetProbability();

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
        <ErrorBoundary label="the disk list">
          <DiskList onselect={handleDiskSelect} />
        </ErrorBoundary>
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
      <ErrorBoundary label="the scan results">
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
        <FileTable {files} selectedInode={selectedFile?.inode_id ?? null} onrowclick={handleRowClick} />
      </div>

      {#if appState === 'complete' || selectedFile}
        <ProbabilityPanel
          report={probReport}
          loading={probLoading}
          error={probError}
          fileName={selectedFile ? (selectedFile.name ?? `recovered_${selectedFile.inode_id}`) : null}
        />
        {#if selectedFile}
          <div class="recover-bar">
            <Button variant="primary" onclick={handleRecoverClick}>
              <HardDriveDownload size={14} strokeWidth={1.5} />
              Recover this file…
            </Button>
          </div>
        {/if}
      {/if}
      </ErrorBoundary>
    </section>
  </div>
</main>

<RecoveryModal
  open={recoverOpen}
  file={selectedFile}
  disk={selectedDisk}
  destPath={recoverDest}
  onclose={closeRecovery}
/>

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
    /* Amber primary (#FFB020) fails text contrast (1.83:1); use the text-safe
       warm amber so the wordmark reads clearly on the light titlebar. */
    color: var(--color-warning);
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
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
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

  /* ── Recover action bar ──────────────────────────────────── */
  .recover-bar {
    display: flex;
    justify-content: flex-end;
    padding: var(--space-3) var(--space-4);
    border-top: 1px solid var(--color-border);
    background: var(--color-bg-layer);
    flex-shrink: 0;
  }
</style>
