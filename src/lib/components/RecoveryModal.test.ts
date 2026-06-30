import type { DeletedFileEntry, DiskInfo, PreflightResult } from '$lib/types';
import { render, screen } from '@testing-library/svelte';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import RecoveryModal from './RecoveryModal.svelte';

// Mock the Tauri IPC surface RecoveryModal uses.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));
// `listen` returns an unlisten fn; resolve to a no-op so attach/detach work.
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock('@tauri-apps/plugin-shell', () => ({
  open: vi.fn(),
}));
// Modal + Button animate via GSAP; jsdom has no layout engine.
vi.mock('gsap', () => ({
  gsap: {
    to: vi.fn(),
    fromTo: vi.fn(),
    set: vi.fn(),
    globalTimeline: { timeScale: vi.fn() },
  },
}));

beforeAll(() => {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
});

const FILE: DeletedFileEntry = {
  inode_id: 42,
  name: 'photo.jpg',
  size_bytes: 40960,
  deleted_at: null,
  extents: [[100, 10]],
  filesystem: 'APFS',
};

const DISK: DiskInfo = {
  id: 'disk0',
  display_name: 'Macintosh HD',
  size_bytes: 500_000_000_000,
  drive_type: 'SSD',
  filesystem: 'APFS',
  mount_points: ['/'],
  encrypted: false,
  trim_enabled: true,
  serial: null,
};

function okPreflight(): PreflightResult {
  return { ok: true, errors: [] };
}

function blockedPreflight(): PreflightResult {
  return {
    ok: false,
    errors: [{ kind: 'SameDisk' }, { kind: 'InsufficientSpace', required: 45000, available: 1000 }],
  };
}

describe('RecoveryModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows the file and destination summary when open', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue(okPreflight());

    render(RecoveryModal, { open: true, file: FILE, disk: DISK, destPath: '/Volumes/Backup' });

    await vi.waitFor(() => {
      expect(screen.getByText('photo.jpg')).toBeTruthy();
      expect(screen.getByText('/Volumes/Backup')).toBeTruthy();
    });
  });

  it('enters confirming state and offers Recover when pre-flight passes', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue(okPreflight());

    render(RecoveryModal, { open: true, file: FILE, disk: DISK, destPath: '/Volumes/Backup' });

    await vi.waitFor(() => {
      expect(screen.getByRole('button', { name: 'Recover' })).toBeTruthy();
    });
  });

  it('shows blocked state with pre-flight error messages', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue(blockedPreflight());

    render(RecoveryModal, { open: true, file: FILE, disk: DISK, destPath: '/Volumes/Same' });

    await vi.waitFor(() => {
      expect(screen.getByText(/same disk as the source/i)).toBeTruthy();
      expect(screen.getByText(/Not enough space/i)).toBeTruthy();
      // No Recover button in the blocked state.
      expect(screen.queryByRole('button', { name: 'Recover' })).toBeNull();
    });
  });

  it('surfaces a pre-flight IPC failure as a blocking error', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockRejectedValue(new Error('device gone'));

    render(RecoveryModal, { open: true, file: FILE, disk: DISK, destPath: '/Volumes/Backup' });

    await vi.waitFor(() => {
      expect(screen.getByText('device gone')).toBeTruthy();
    });
  });

  it('renders nothing when closed', () => {
    render(RecoveryModal, { open: false, file: FILE, disk: DISK, destPath: null });
    expect(screen.queryByText('photo.jpg')).toBeNull();
  });
});
