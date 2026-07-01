import type { DeletedFileEntry } from '$lib/types';
import { fireEvent, render } from '@testing-library/svelte';
import { beforeAll, describe, expect, it, vi } from 'vitest';
import FileTable from './FileTable.svelte';

// jsdom lacks ResizeObserver (FileTable observes its scroll container).
beforeAll(() => {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
});

function entry(over: Partial<DeletedFileEntry> = {}): DeletedFileEntry {
  return {
    inode_id: 1,
    name: 'photo.jpg',
    size_bytes: 2048,
    deleted_at: 1_700_000_000,
    extents: [],
    filesystem: 'APFS',
    ...over,
  };
}

describe('FileTable', () => {
  it('shows the empty state when there are no files', () => {
    const { getByText, container } = render(FileTable, { files: [] });
    expect(getByText('Scan a disk to see deleted files.')).toBeTruthy();
    // No data rows in the empty state.
    expect(container.querySelector('.file-table__row')).toBeNull();
  });

  it('renders rows and a pluralised footer count', () => {
    const files = [entry({ inode_id: 1 }), entry({ inode_id: 2, name: 'clip.mov' })];
    const { container, getByText } = render(FileTable, { files });
    const rows = container.querySelectorAll('.file-table__row');
    expect(rows.length).toBe(2);
    expect(getByText('2 files found')).toBeTruthy();
  });

  it('uses singular "file" for exactly one entry', () => {
    const { getByText } = render(FileTable, { files: [entry()] });
    expect(getByText('1 file found')).toBeTruthy();
  });

  it('falls back to a synthetic name when the directory entry was zeroed', () => {
    const { getByText } = render(FileTable, { files: [entry({ inode_id: 42, name: null })] });
    expect(getByText('recovered_42')).toBeTruthy();
  });

  it('renders an em dash when the deletion timestamp is unavailable', () => {
    const { container } = render(FileTable, {
      files: [entry({ name: null, deleted_at: null })],
    });
    const dateCol = container.querySelector('.file-table__row .col--date');
    expect(dateCol?.textContent?.trim()).toBe('—');
  });

  it('fires onrowclick with the clicked file', async () => {
    const onrowclick = vi.fn();
    const target = entry({ inode_id: 7, name: 'song.mp3' });
    const { container } = render(FileTable, { files: [target], onrowclick });
    const row = container.querySelector('.file-table__row');
    expect(row).not.toBeNull();
    if (!row) throw new Error('row not found');
    await fireEvent.click(row);
    expect(onrowclick).toHaveBeenCalledWith(target);
  });

  it('marks the selected row via aria-selected', () => {
    const files = [entry({ inode_id: 1 }), entry({ inode_id: 2 })];
    const { container } = render(FileTable, { files, selectedInode: 2 });
    const selected = container.querySelectorAll('[aria-selected="true"]');
    expect(selected.length).toBe(1);
  });

  it('windows the row list for large inputs (only a subset is in the DOM)', () => {
    // 5000 rows but a 400px viewport + buffer → far fewer rendered nodes.
    const files = Array.from({ length: 5000 }, (_, i) => entry({ inode_id: i + 1 }));
    const { container } = render(FileTable, { files });
    const rendered = container.querySelectorAll('.file-table__row').length;
    expect(rendered).toBeGreaterThan(0);
    expect(rendered).toBeLessThan(200);
    // Footer still reports the full count.
    expect(container.querySelector('.file-table__footer')?.textContent).toContain('5,000');
  });
});
