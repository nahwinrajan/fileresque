import { describe, expect, it } from 'vitest';
import { formatBytes, formatDate } from './format';

// ---------------------------------------------------------------------------
// formatBytes
// ---------------------------------------------------------------------------

describe('formatBytes', () => {
  interface Case {
    name: string;
    bytes: number;
    decimals?: number;
    expected: string;
  }

  const cases: Case[] = [
    { name: 'happy_path_zero', bytes: 0, expected: '0 B' },
    { name: 'branch_negative', bytes: -1, expected: '0 B' },
    { name: 'happy_path_sub_kb', bytes: 500, expected: '500 B' },
    { name: 'happy_path_one_byte', bytes: 1, expected: '1 B' },
    { name: 'happy_path_kb_boundary', bytes: 1024, expected: '1 KB' },
    { name: 'happy_path_mb_boundary', bytes: 1024 * 1024, expected: '1 MB' },
    { name: 'happy_path_gb_boundary', bytes: 1024 * 1024 * 1024, expected: '1 GB' },
    { name: 'happy_path_tb_boundary', bytes: 1024 ** 4, expected: '1 TB' },
    { name: 'happy_path_custom_decimals', bytes: 1500, decimals: 2, expected: '1.46 KB' },
    { name: 'happy_path_zero_decimals', bytes: 2048, decimals: 0, expected: '2 KB' },
    { name: 'branch_fractional_gb', bytes: 1.5 * 1024 ** 3, decimals: 1, expected: '1.5 GB' },
  ];

  for (const c of cases) {
    it(c.name, () => {
      const actual =
        c.decimals !== undefined ? formatBytes(c.bytes, c.decimals) : formatBytes(c.bytes);
      expect(actual, `FAILED case: ${c.name}`).toBe(c.expected);
    });
  }
});

// ---------------------------------------------------------------------------
// formatDate
// ---------------------------------------------------------------------------

describe('formatDate', () => {
  interface Case {
    name: string;
    input: number | null;
    expected: string | null; // null means "just check it's not '—'"
  }

  const cases: Case[] = [
    { name: 'happy_path_null', input: null, expected: '—' },
    { name: 'branch_epoch_zero', input: 0, expected: null },
    { name: 'branch_positive_timestamp', input: 1_700_000_000, expected: null },
  ];

  for (const c of cases) {
    it(c.name, () => {
      const actual = formatDate(c.input);
      if (c.expected !== null) {
        expect(actual, `FAILED case: ${c.name}`).toBe(c.expected);
      } else {
        // Valid timestamps must produce a non-empty string that is not the null sentinel.
        expect(actual, `FAILED case: ${c.name}`).not.toBe('—');
        expect(actual.length, `FAILED case: ${c.name} — empty string`).toBeGreaterThan(0);
      }
    });
  }
});
