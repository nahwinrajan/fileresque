<script lang="ts">
import { open as shellOpen } from '@tauri-apps/plugin-shell';
import { ShieldAlert } from 'lucide-svelte';
import Button from './Button.svelte';
import Modal from './Modal.svelte';

interface Props {
  open: boolean;
  ondismissed?: () => void;
}

const { open, ondismissed }: Props = $props();

function handleOpenSettings(): void {
  shellOpen('x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles');
}

function handleSkip(): void {
  ondismissed?.();
}
</script>

<Modal {open} onclose={handleSkip} title="Full Disk Access Required">
  <div class="permission-modal-content">
    <div class="icon-row">
      <ShieldAlert size={32} color="var(--color-warning)" strokeWidth={1.5} aria-hidden="true" />
    </div>
    <p class="body-text">
      FileResque needs Full Disk Access to scan your drives for deleted files.
      Without it, recovery isn't possible.
    </p>
    <ol class="steps" aria-label="Steps to grant Full Disk Access">
      <li>
        Open <strong>System Settings</strong> &rarr; <strong>Privacy &amp; Security</strong>
        &rarr; <strong>Full Disk Access</strong>
      </li>
      <li>Click the <strong>+</strong> button and add FileResque</li>
      <li>Restart FileResque</li>
    </ol>
    <div class="actions">
      <Button variant="primary" onclick={handleOpenSettings}>Open System Settings</Button>
      <Button variant="ghost" onclick={handleSkip}>Skip for now</Button>
    </div>
  </div>
</Modal>

<style>
  .permission-modal-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
  }

  .icon-row {
    display: flex;
    justify-content: center;
    padding-bottom: var(--space-2);
  }

  .body-text {
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
    line-height: var(--line-height-relaxed);
    margin: 0;
  }

  .steps {
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
    line-height: var(--line-height-relaxed);
    padding-left: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
  }

  .steps strong {
    color: var(--color-text-dim);
  }

  .actions {
    display: flex;
    gap: var(--space-3);
    justify-content: flex-end;
    padding-top: var(--space-2);
  }
</style>
