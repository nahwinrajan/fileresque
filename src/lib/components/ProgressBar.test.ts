import { render } from '@testing-library/svelte';
import { beforeAll, describe, expect, it, vi } from 'vitest';
import ProgressBar from './ProgressBar.svelte';

// Animation is irrelevant to the values under test.
vi.mock('gsap', () => ({
  gsap: { to: vi.fn(), fromTo: vi.fn(), set: vi.fn() },
}));

beforeAll(() => {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      // Reduced motion ON → fill width is set synchronously (no gsap tween),
      // which keeps the DOM deterministic for assertions.
      matches: true,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
});

describe('ProgressBar', () => {
  it('exposes value/min/max on the progressbar role', () => {
    const { container } = render(ProgressBar, { value: 30, max: 120, label: 'Copying' });
    const bar = container.querySelector('[role="progressbar"]');
    expect(bar?.getAttribute('aria-valuenow')).toBe('30');
    expect(bar?.getAttribute('aria-valuemin')).toBe('0');
    expect(bar?.getAttribute('aria-valuemax')).toBe('120');
    expect(bar?.getAttribute('aria-label')).toBe('Copying');
  });

  it('rounds the displayed percent when showPercent is set', () => {
    // 1/3 → 33.33% → rounds to 33.
    const { getByText } = render(ProgressBar, { value: 1, max: 3, showPercent: true });
    expect(getByText('33%')).toBeTruthy();
  });

  it('clamps over-100% input to 100', () => {
    const { getByText } = render(ProgressBar, { value: 999, max: 100, showPercent: true });
    expect(getByText('100%')).toBeTruthy();
  });

  it('reports 0% when max is zero (guards divide-by-zero)', () => {
    const { getByText } = render(ProgressBar, { value: 50, max: 0, showPercent: true });
    expect(getByText('0%')).toBeTruthy();
  });

  it('renders the label and applies the variant fill class', () => {
    const { getByText, container } = render(ProgressBar, {
      value: 10,
      max: 100,
      label: 'Recovering',
      variant: 'danger',
    });
    expect(getByText('Recovering')).toBeTruthy();
    expect(container.querySelector('.progress__fill--danger')).not.toBeNull();
  });

  it('omits the header when neither label nor showPercent is given', () => {
    const { container } = render(ProgressBar, { value: 10, max: 100 });
    expect(container.querySelector('.progress__header')).toBeNull();
  });
});
