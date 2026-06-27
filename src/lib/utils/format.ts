/**
 * FileResque — formatting utilities
 *
 * Pure functions with no side effects. Safe to call in both browser and SSR contexts.
 */

const SIZE_UNITS = ['B', 'KB', 'MB', 'GB', 'TB'] as const;
const SIZE_BASE = 1024;

/**
 * Format a byte count into a human-readable string with the appropriate unit.
 *
 * @param bytes    Raw byte count. Negative values are treated as zero.
 * @param decimals Number of decimal places (default: 1).
 *
 * @example
 *   formatBytes(0)          // '0 B'
 *   formatBytes(1024)       // '1 KB'
 *   formatBytes(1500, 2)    // '1.46 KB'
 */
export function formatBytes(bytes: number, decimals = 1): string {
  if (bytes <= 0) {
    return '0 B';
  }

  const exponent = Math.min(
    Math.floor(Math.log(bytes) / Math.log(SIZE_BASE)),
    SIZE_UNITS.length - 1
  );
  const value = bytes / SIZE_BASE ** exponent;

  return `${Number.parseFloat(value.toFixed(decimals))} ${SIZE_UNITS[exponent]}`;
}

/**
 * Format a Unix epoch timestamp (seconds) into a localised short date string.
 *
 * Returns an em dash ('—') when the timestamp is null (deletion time unknown).
 *
 * @param epochSeconds Seconds since Unix epoch, or null.
 *
 * @example
 *   formatDate(null)        // '—'
 *   formatDate(0)           // locale-dependent date string (Jan 1, 1970)
 */
export function formatDate(epochSeconds: number | null): string {
  if (epochSeconds === null) {
    return '—';
  }

  return new Date(epochSeconds * 1000).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}
