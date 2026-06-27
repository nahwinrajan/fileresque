import type { DiskInfo } from '$lib/types';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import DiskList from './DiskList.svelte';

// Mock @tauri-apps/api/core — invoke is called by DiskList on mount.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock GSAP — Card.svelte and Button.svelte animate via gsap; jsdom has no
// layout engine so GSAP must be stubbed out.
vi.mock('gsap', () => ({
  gsap: {
    to: vi.fn(),
    fromTo: vi.fn(),
    set: vi.fn(),
    globalTimeline: { timeScale: vi.fn() },
  },
}));

// jsdom does not implement window.matchMedia; provide a stub that returns
// prefers-reduced-motion: false so GSAP guard logic stays inactive.
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

const mockDisk: DiskInfo = {
  id: 'disk0',
  display_name: 'Apple SSD',
  size_bytes: 512_000_000_000,
  drive_type: 'NVMe',
  filesystem: 'APFS',
  mount_points: ['/'],
  encrypted: true,
  trim_enabled: true,
  serial: 'ABC123',
};

describe('DiskList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows loading skeleton on mount while invoke is pending', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    // Never resolves — keeps the component in the loading state.
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}));

    const { container } = render(DiskList);

    const skeletons = container.querySelectorAll('.skeleton-card');
    expect(skeletons.length).toBe(3);
  });

  it('shows disk cards when get_disks resolves with data', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue([mockDisk]);

    render(DiskList);

    await vi.waitFor(() => {
      expect(screen.getByText('Apple SSD')).toBeTruthy();
    });
    expect(screen.getByText('NVMe')).toBeTruthy();
    expect(screen.getByText('APFS')).toBeTruthy();
    expect(screen.getByText('Encrypted')).toBeTruthy();
    expect(screen.getByText('TRIM')).toBeTruthy();
  });

  it('shows empty state when get_disks returns an empty array', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue([]);

    render(DiskList);

    await vi.waitFor(() => {
      expect(screen.getByText('No disks found.')).toBeTruthy();
    });
  });

  it('shows permission-specific error message when invoke rejects with permission error', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockRejectedValue(new Error('Permission denied'));

    render(DiskList);

    await vi.waitFor(() => {
      expect(screen.getByText(/Full Disk Access is required/)).toBeTruthy();
    });
  });

  it('shows generic error message for non-permission errors', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockRejectedValue(new Error('Unknown IO error'));

    render(DiskList);

    await vi.waitFor(() => {
      expect(screen.getByText(/Could not enumerate disks/)).toBeTruthy();
    });
  });

  it('retry button calls get_disks again and shows disks on success', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke)
      .mockRejectedValueOnce(new Error('IO error'))
      .mockResolvedValueOnce([mockDisk]);

    render(DiskList);

    await vi.waitFor(() => screen.getByText(/Could not enumerate disks/));

    const retryBtn = screen.getByText('Retry').closest('button');
    if (!retryBtn) throw new Error('Retry button not found in DOM');
    fireEvent.click(retryBtn);

    await vi.waitFor(() => expect(screen.getByText('Apple SSD')).toBeTruthy());
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it('calls onselect with the clicked DiskInfo', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue([mockDisk]);
    const onselect = vi.fn();

    render(DiskList, { onselect });

    await vi.waitFor(() => screen.getByText('Apple SSD'));

    const diskBtn = screen.getByText('Apple SSD').closest('button');
    if (!diskBtn) throw new Error('Disk button not found in DOM');
    fireEvent.click(diskBtn);

    expect(onselect).toHaveBeenCalledWith(mockDisk);
  });

  it('does not render a filesystem badge when filesystem is Unknown', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const unknownFsDisk: DiskInfo = { ...mockDisk, filesystem: 'Unknown' };
    vi.mocked(invoke).mockResolvedValue([unknownFsDisk]);

    render(DiskList);

    await vi.waitFor(() => screen.getByText('Apple SSD'));
    expect(screen.queryByText('Unknown')).toBeNull();
  });

  it('formats size_bytes into a human-readable size string', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue([mockDisk]);

    render(DiskList);

    await vi.waitFor(() => {
      // 512_000_000_000 bytes ≈ 476.8 GB (binary) or 512.0 GB (decimal)
      expect(screen.getByText(/\d+(\.\d+)? [KMGT]B/)).toBeTruthy();
    });
  });
});
