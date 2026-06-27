import { render, screen } from '@testing-library/svelte';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import PermissionGate from './PermissionGate.svelte';

// Mock @tauri-apps/api/core — invoke is called by PermissionGate on mount.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock @tauri-apps/plugin-shell — imported by PermissionModal; not invoked
// during these tests (no button clicks) but must resolve for the import to succeed.
vi.mock('@tauri-apps/plugin-shell', () => ({
  open: vi.fn(),
}));

// Mock GSAP — Modal.svelte and Button.svelte animate via gsap; jsdom has no
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
// prefers-reduced-motion: false (no animation suppression in tests).
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

describe('PermissionGate', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows modal when check_disk_access returns false', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue(false);

    render(PermissionGate);

    await vi.waitFor(() => {
      expect(screen.getByText('Full Disk Access Required')).toBeTruthy();
    });
  });

  it('does not show modal when check_disk_access returns true', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue(true);

    render(PermissionGate);

    await vi.waitFor(() => {
      expect(screen.queryByText('Full Disk Access Required')).toBeNull();
    });
  });

  it('does not show modal when check_disk_access errors', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockRejectedValue(new Error('IPC error'));

    render(PermissionGate);

    await vi.waitFor(() => {
      expect(screen.queryByText('Full Disk Access Required')).toBeNull();
    });
  });
});
