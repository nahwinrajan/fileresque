<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import PermissionModal from './PermissionModal.svelte';

let permissionGranted = $state<boolean | null>(null);
let modalOpen = $state(false);

$effect(() => {
  invoke<boolean>('check_disk_access')
    .then((granted) => {
      permissionGranted = granted;
      if (!granted) {
        modalOpen = true;
      }
    })
    .catch(() => {
      // If the check errors, assume access is granted — disk enumeration
      // will surface the real error with a descriptive message.
      permissionGranted = true;
    });
});

function handleDismissed(): void {
  modalOpen = false;
}
</script>

<!--
  PermissionGate renders no visible DOM of its own — it only conditionally
  shows PermissionModal when Full Disk Access has not been granted.
  Consumers place <PermissionGate /> at the app root so the modal overlays
  the entire UI (z-index: var(--z-overlay) is set inside Modal).
-->
<PermissionModal open={modalOpen} ondismissed={handleDismissed} />
