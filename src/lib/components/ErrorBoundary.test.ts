import { render } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { beforeAll, describe, expect, it, vi } from 'vitest';
import ErrorBoundary from './ErrorBoundary.svelte';

// jsdom does not implement window.matchMedia; provide a stub (layout uses it).
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
});

const okChild = createRawSnippet(() => ({
  render: () => '<p data-testid="child-ok">all good</p>',
}));

const throwingChild = createRawSnippet(() => ({
  render: (): string => {
    throw new Error('boom');
  },
}));

describe('ErrorBoundary', () => {
  it('renders children when nothing throws', () => {
    const { getByTestId, container } = render(ErrorBoundary, { children: okChild });
    expect(getByTestId('child-ok').textContent).toBe('all good');
    // No fallback alert in the happy path.
    expect(container.querySelector('[role="alert"]')).toBeNull();
  });

  it('shows a recoverable fallback when a child throws', () => {
    // Boundary logs the error to the console on purpose; silence it in the test.
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const { container } = render(ErrorBoundary, {
      children: throwingChild,
      label: 'the disk list',
    });

    const alert = container.querySelector('[role="alert"]');
    expect(alert).not.toBeNull();
    expect(alert?.textContent).toContain('the disk list');
    // A retry control is offered so the user need not restart the app.
    expect(container.querySelector('.boundary__retry')).not.toBeNull();
    spy.mockRestore();
  });
});
