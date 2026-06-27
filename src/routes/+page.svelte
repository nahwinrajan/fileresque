<script lang="ts">
// P0-T01 placeholder — replaced by P1-T03 DiskList view
import { PermissionGate } from '$lib/components/index.js';
</script>

<PermissionGate />

<main class="app-shell">
  <div class="titlebar" data-tauri-drag-region aria-label="FileResque title bar">
    <span class="wordmark" aria-label="FileResque">
      File<strong>Resque</strong>
    </span>
  </div>
  <div class="content" role="main">
    <p>FileResque is initialising…</p>
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

  .content {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
</style>
