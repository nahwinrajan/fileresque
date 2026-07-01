import type { DeletedFileEntry, DiskInfo, ProbabilityReport } from '$lib/types';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';

// Capture Tauri event listeners so tests can drive scan-lifecycle events.
const h = vi.hoisted(() => ({
  listeners: {} as Record<string, (e: { payload: unknown }) => void>,
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((event: string, cb: (e: { payload: unknown }) => void) => {
    h.listeners[event] = cb;
    return Promise.resolve(() => {});
  }),
}));
vi.mock('@tauri-apps/plugin-shell', () => ({ open: vi.fn() }));
vi.mock('gsap', () => ({
  gsap: {
    to: vi.fn(),
    fromTo: vi.fn(),
    set: vi.fn(),
    globalTimeline: { timeScale: vi.fn() },
  },
}));

const disk: DiskInfo = {
  id: 'disk2',
  display_name: 'Macintosh HD',
  size_bytes: 512_000_000_000,
  drive_type: 'SSD',
  filesystem: 'APFS',
  mount_points: ['/'],
  encrypted: false,
  trim_enabled: true,
  serial: null,
};

const fileEntry: DeletedFileEntry = {
  inode_id: 101,
  name: 'report.pdf',
  size_bytes: 4096,
  deleted_at: 1_700_000_000,
  extents: [[10, 2]],
  filesystem: 'APFS',
};

const probReport: ProbabilityReport = {
  tier: 'High',
  free_blocks_pct: 80,
  trim_active: false,
  blocks_zeroed: false,
  estimated_recoverable_bytes: 4096,
  warnings: [],
};

beforeAll(() => {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
});

async function invoke() {
  return (await import('@tauri-apps/api/core')).invoke;
}

beforeEach(async () => {
  vi.clearAllMocks();
  for (const k of Object.keys(h.listeners)) delete h.listeners[k];
  vi.mocked(await invoke()).mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case 'check_disk_access':
        return true;
      case 'get_disks':
        return [disk];
      case 'check_probability':
        return probReport;
      case 'pick_destination_folder':
        return null; // user cancels the picker
      case 'preflight_recovery':
        return { ok: true, errors: [] };
      default:
        return undefined;
    }
  });
});

/** Select the disk and start a scan; returns once scan listeners are live. */
async function startScan() {
  render(Page);
  const diskBtn = await screen.findByRole('button', { name: /Select Macintosh HD/ });
  await fireEvent.click(diskBtn);
  const scanBtn = await screen.findByRole('button', { name: /Scan Macintosh HD/ });
  await fireEvent.click(scanBtn);
  await waitFor(() => expect(h.listeners['scan:complete']).toBeDefined());
}

describe('+page orchestration', () => {
  it('scans a disk, streams found files, and shows a completion summary', async () => {
    await startScan();
    const inv = await invoke();
    expect(inv).toHaveBeenCalledWith('start_scan', { diskId: 'disk2' });

    h.listeners['scan:file_found']({ payload: fileEntry });
    h.listeners['scan:progress']({ payload: { files_found: 1 } });
    h.listeners['scan:complete']({ payload: { total_found: 1, duration_ms: 1500 } });

    await waitFor(() => {
      const summary = document.querySelector('.scan-summary__text');
      const text = summary?.textContent?.replace(/\s+/g, ' ').trim();
      expect(text).toContain('1 deleted file found in');
      // formatDuration(1500) → "1.5s" (exercises the ≥1000ms branch).
      expect(text).toContain('1.5s');
    });
  });

  it('requests a probability report when a file row is clicked', async () => {
    await startScan();
    h.listeners['scan:file_found']({ payload: fileEntry });
    h.listeners['scan:complete']({ payload: { total_found: 1, duration_ms: 200 } });

    const row = await screen.findByRole('row', { name: /report\.pdf/ });
    await fireEvent.click(row);

    const inv = await invoke();
    await waitFor(() => {
      expect(inv).toHaveBeenCalledWith('check_probability', { entry: fileEntry, disk });
    });

    // The recover action appears once a file is selected; clicking it opens the
    // native picker (mocked to cancel → no modal, exercises the early return).
    const recoverBtn = await screen.findByRole('button', { name: /Recover this file/ });
    await fireEvent.click(recoverBtn);
    await waitFor(() => expect(inv).toHaveBeenCalledWith('pick_destination_folder'));
  });

  it('surfaces a scan error with a retry affordance', async () => {
    await startScan();
    h.listeners['scan:error']({
      payload: { message: 'A disk read error occurred.', recoverable: false },
    });

    await waitFor(() => {
      expect(screen.getByText(/Scan failed: A disk read error occurred\./)).toBeTruthy();
      expect(screen.getByRole('button', { name: /Retry/ })).toBeTruthy();
    });
  });

  it('surfaces a mid-scan disk disconnection', async () => {
    await startScan();
    h.listeners['disk:disconnected']({
      payload: { disk_id: 'disk2', message: 'The source disk was disconnected.' },
    });

    await waitFor(() => {
      expect(screen.getByText(/The source disk was disconnected\./)).toBeTruthy();
    });
  });

  it('cancels an in-progress scan', async () => {
    await startScan();
    const cancelBtn = await screen.findByRole('button', { name: /Cancel scan/ });
    await fireEvent.click(cancelBtn);
    const inv = await invoke();
    expect(inv).toHaveBeenCalledWith('cancel_scan');
  });
});
