import type { ProbabilityReport, ProbabilityTier } from '$lib/types';
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import ProbabilityPanel from './ProbabilityPanel.svelte';

function report(
  tier: ProbabilityTier,
  overrides: Partial<ProbabilityReport> = {}
): ProbabilityReport {
  return {
    tier,
    free_blocks_pct: 90,
    trim_active: false,
    blocks_zeroed: false,
    estimated_recoverable_bytes: 1024,
    warnings: [],
    ...overrides,
  };
}

describe('ProbabilityPanel', () => {
  it('renders the empty state by default', () => {
    render(ProbabilityPanel, {});
    expect(screen.getByText(/select a file to assess/i)).toBeTruthy();
  });

  it('renders a loading state', () => {
    render(ProbabilityPanel, { loading: true });
    expect(screen.getByText(/assessing recoverability/i)).toBeTruthy();
  });

  it('renders an error state', () => {
    render(ProbabilityPanel, { error: 'Cannot open /dev/rdisk0' });
    expect(screen.getByRole('alert').textContent).toContain('Cannot open /dev/rdisk0');
  });

  it('renders a High tier report', () => {
    render(ProbabilityPanel, { report: report('High') });
    expect(screen.getByLabelText(/probability tier: high/i)).toBeTruthy();
    expect(
      screen.getByRole('progressbar', { name: /free blocks/i }).getAttribute('aria-valuenow')
    ).toBe('90');
  });

  it('renders a Medium tier report', () => {
    render(ProbabilityPanel, { report: report('Medium', { free_blocks_pct: 60 }) });
    expect(screen.getByLabelText(/probability tier: medium/i)).toBeTruthy();
  });

  it('renders a Low tier report with warnings', () => {
    render(ProbabilityPanel, {
      report: report('Low', {
        free_blocks_pct: 5,
        trim_active: true,
        warnings: ['TRIM is active on this SSD; freed blocks are likely discarded.'],
      }),
    });
    expect(screen.getByLabelText(/probability tier: low/i)).toBeTruthy();
    expect(screen.getByText(/trim is active/i)).toBeTruthy();
  });

  it('shows the file name when provided', () => {
    render(ProbabilityPanel, { report: report('High'), fileName: 'photo.jpg' });
    expect(screen.getByText('photo.jpg')).toBeTruthy();
  });
});
