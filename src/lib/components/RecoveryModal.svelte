<script lang="ts">
import Button from '$lib/components/Button.svelte';
import Modal from '$lib/components/Modal.svelte';
import ProgressBar from '$lib/components/ProgressBar.svelte';
import type {
  DeletedFileEntry,
  DiskDisconnectedEvent,
  DiskInfo,
  PreflightError,
  PreflightResult,
  RecoveryCompleteEvent,
  RecoveryFileCompleteEvent,
  RecoveryProgressEvent,
} from '$lib/types';
import { formatBytes } from '$lib/utils/format';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { AlertTriangle, CheckCircle2, FolderOpen, ShieldAlert } from 'lucide-svelte';

const {
  open = false,
  file = null,
  disk = null,
  destPath = null,
  onclose,
}: {
  open?: boolean;
  file?: DeletedFileEntry | null;
  disk?: DiskInfo | null;
  destPath?: string | null;
  onclose?: () => void;
} = $props();

// ── Flow state machine ────────────────────────────────────────────────────────
// preflight → (blocked | confirming) → recovering → done
type Phase = 'preflight' | 'blocked' | 'confirming' | 'recovering' | 'done';

let phase = $state<Phase>('preflight');
let preflight = $state<PreflightResult | null>(null);
let bytesWritten = $state(0);
let summary = $state<RecoveryCompleteEvent | null>(null);
let lastResult = $state<RecoveryFileCompleteEvent | null>(null);
let actionError = $state<string | null>(null);

type UnlistenFn = () => void;
let unlisteners: UnlistenFn[] = [];

const fileLabel = $derived(file ? (file.name ?? `recovered_${file.inode_id}`) : '');

// Run pre-flight whenever the modal opens with a destination.
$effect(() => {
  if (!open || !file || !disk || !destPath) return;
  resetState();
  void runPreflight();
  return detachListeners;
});

function resetState(): void {
  phase = 'preflight';
  preflight = null;
  bytesWritten = 0;
  summary = null;
  lastResult = null;
  actionError = null;
}

async function runPreflight(): Promise<void> {
  if (!file || !disk || !destPath) return;
  try {
    const result = await invoke<PreflightResult>('preflight_recovery', {
      entries: [file],
      source: disk,
      destPath,
    });
    preflight = result;
    phase = result.ok ? 'confirming' : 'blocked';
  } catch (err) {
    actionError = err instanceof Error ? err.message : String(err);
    phase = 'blocked';
  }
}

async function attachListeners(): Promise<void> {
  unlisteners = await Promise.all([
    listen<RecoveryProgressEvent>('recovery:progress', ({ payload }) => {
      bytesWritten = payload.bytes_written;
    }),
    listen<RecoveryFileCompleteEvent>('recovery:file_complete', ({ payload }) => {
      lastResult = payload;
    }),
    listen<RecoveryCompleteEvent>('recovery:complete', ({ payload }) => {
      summary = payload;
      phase = 'done';
    }),
    // Source disk pulled mid-recovery (P5-T03): the engine aborts, so end the
    // flow with the friendly reason rather than a stalled progress bar.
    listen<DiskDisconnectedEvent>('disk:disconnected', ({ payload }) => {
      actionError = payload.message;
      phase = 'done';
    }),
  ]);
}

function detachListeners(): void {
  for (const fn of unlisteners) fn();
  unlisteners = [];
}

async function startRecovery(): Promise<void> {
  if (!file || !disk || !destPath) return;
  actionError = null;
  bytesWritten = 0;
  phase = 'recovering';
  detachListeners();
  await attachListeners();
  try {
    await invoke('recover_files', { entries: [file], source: disk, destPath });
  } catch (err) {
    actionError = err instanceof Error ? err.message : String(err);
    phase = 'done';
  }
}

async function cancelRecovery(): Promise<void> {
  try {
    await invoke('cancel_recovery');
  } catch {
    // best-effort
  }
}

async function openDestination(): Promise<void> {
  // Open the destination *folder* (covered by the existing shell:allow-open
  // capability); revealing a single file needs a separate permission.
  if (!destPath) return;
  try {
    const { open } = await import('@tauri-apps/plugin-shell');
    await open(destPath);
  } catch {
    // non-fatal: opening the folder is a convenience
  }
}

// Backdrop/escape close is ignored while actively recovering; the user must use
// the explicit Cancel button so an in-flight write is never silently abandoned.
function handleClose(): void {
  if (phase === 'recovering') return;
  detachListeners();
  onclose?.();
}

const PREFLIGHT_MESSAGES: Record<PreflightError['kind'], string> = {
  SameDisk: 'Destination is on the same disk as the source. Choose a different drive.',
  InsufficientSpace: 'Not enough free space at the destination.',
  DestinationNotWritable: 'The destination folder is not writable.',
  SourceNotReadable: 'The source disk is no longer readable.',
};

function preflightMessage(err: PreflightError): string {
  if (err.kind === 'InsufficientSpace') {
    return `Not enough space: needs ${formatBytes(err.required)}, only ${formatBytes(err.available)} free.`;
  }
  return PREFLIGHT_MESSAGES[err.kind];
}

const progressMax = $derived(file && file.size_bytes > 0 ? file.size_bytes : 1);
</script>

<Modal {open} title="Recover file" onclose={handleClose}>
  <div class="recovery">
    <!-- File + destination summary (always shown) -->
    <div class="recovery__meta">
      <div class="recovery__row">
        <span class="recovery__key">File</span>
        <span class="recovery__val recovery__val--mono" title={fileLabel}>{fileLabel}</span>
      </div>
      <div class="recovery__row">
        <span class="recovery__key">Size</span>
        <span class="recovery__val">{formatBytes(file?.size_bytes ?? 0)}</span>
      </div>
      <div class="recovery__row">
        <span class="recovery__key">Destination</span>
        <span class="recovery__val recovery__val--mono" title={destPath ?? ''}>{destPath ?? '—'}</span>
      </div>
    </div>

    {#if phase === 'preflight'}
      <p class="recovery__status" role="status" aria-live="polite">Running pre-flight checks…</p>
    {:else if phase === 'blocked'}
      <div class="recovery__blocked" role="alert">
        <ShieldAlert size={16} strokeWidth={1.5} aria-hidden="true" />
        <div>
          <p class="recovery__blocked-title">Recovery cannot proceed</p>
          <ul class="recovery__errors">
            {#if actionError}
              <li>{actionError}</li>
            {/if}
            {#each preflight?.errors ?? [] as err (err.kind)}
              <li>{preflightMessage(err)}</li>
            {/each}
          </ul>
        </div>
      </div>
    {:else if phase === 'confirming'}
      <p class="recovery__status">
        Pre-flight passed. The file will be written to the destination and verified with a SHA-256
        digest.
      </p>
    {:else if phase === 'recovering'}
      <div class="recovery__progress">
        <ProgressBar
          value={bytesWritten}
          max={progressMax}
          label="Recovering {fileLabel}"
          showPercent
        />
        <span class="recovery__bytes">{formatBytes(bytesWritten)} / {formatBytes(file?.size_bytes ?? 0)}</span>
      </div>
    {:else if phase === 'done'}
      {@render doneView()}
    {/if}
  </div>

  {#snippet footer()}
    {#if phase === 'confirming'}
      <Button variant="secondary" onclick={handleClose}>Cancel</Button>
      <Button variant="primary" onclick={startRecovery}>Recover</Button>
    {:else if phase === 'recovering'}
      <Button variant="danger" onclick={cancelRecovery}>Cancel recovery</Button>
    {:else if phase === 'done'}
      <Button variant="secondary" onclick={openDestination}>
        <FolderOpen size={14} strokeWidth={1.5} />
        Open folder
      </Button>
      <Button variant="primary" onclick={handleClose}>Done</Button>
    {:else}
      <Button variant="secondary" onclick={handleClose}>Close</Button>
    {/if}
  {/snippet}
</Modal>

{#snippet doneView()}
  {#if actionError}
    <div class="recovery__blocked" role="alert">
      <AlertTriangle size={16} strokeWidth={1.5} aria-hidden="true" />
      <span>{actionError}</span>
    </div>
  {:else if summary?.cancelled}
    <p class="recovery__status">Recovery cancelled. No partial file was left behind.</p>
  {:else if summary && summary.failed > 0}
    <div class="recovery__blocked" role="alert">
      <AlertTriangle size={16} strokeWidth={1.5} aria-hidden="true" />
      <span>Recovery failed{lastResult?.error ? `: ${lastResult.error}` : '.'}</span>
    </div>
  {:else}
    <div class="recovery__success" role="status">
      <CheckCircle2 size={18} strokeWidth={1.5} aria-hidden="true" />
      <div>
        <p class="recovery__success-title">
          {summary && summary.partial > 0 ? 'Recovered with bad sectors' : 'File recovered'}
        </p>
        {#if lastResult?.final_path}
          <p class="recovery__success-path">{lastResult.final_path}</p>
        {/if}
        {#if (lastResult?.blocks_skipped ?? 0) > 0}
          <p class="recovery__warn">
            {lastResult?.blocks_skipped} block(s) were unreadable and zero-filled.
          </p>
        {/if}
      </div>
    </div>
  {/if}
{/snippet}

<style>
  .recovery {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    font-size: var(--font-size-sm);
  }

  .recovery__meta {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--color-bg-layer);
    border-radius: var(--radius-md);
  }

  .recovery__row {
    display: flex;
    gap: var(--space-3);
    align-items: baseline;
  }

  .recovery__key {
    flex-shrink: 0;
    width: 88px;
    font-size: var(--font-size-xs);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-secondary);
  }

  .recovery__val {
    color: var(--color-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .recovery__val--mono {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--color-text-dim);
  }

  .recovery__status {
    margin: 0;
    color: var(--color-text-secondary);
    line-height: var(--line-height-relaxed);
  }

  .recovery__progress {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .recovery__bytes {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--color-text-dim);
    align-self: flex-end;
  }

  .recovery__blocked,
  .recovery__success {
    display: flex;
    gap: var(--space-3);
    align-items: flex-start;
    padding: var(--space-3);
    border-radius: var(--radius-md);
  }

  .recovery__blocked {
    background: color-mix(in srgb, var(--color-danger) 8%, transparent);
    color: var(--color-danger);
  }

  .recovery__success {
    background: color-mix(in srgb, var(--color-success) 8%, transparent);
    color: var(--color-success);
  }

  .recovery__blocked-title,
  .recovery__success-title {
    margin: 0 0 var(--space-1);
    font-weight: var(--font-weight-medium);
  }

  .recovery__errors {
    margin: 0;
    padding-left: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .recovery__success-path,
  .recovery__warn {
    margin: var(--space-1) 0 0;
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--color-text-dim);
    word-break: break-all;
  }

  .recovery__warn {
    color: var(--color-warning);
    font-family: var(--font-family);
  }
</style>
