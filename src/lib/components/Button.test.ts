import { fireEvent, render } from '@testing-library/svelte';
import { beforeAll, describe, expect, it, vi } from 'vitest';
import Button from './Button.svelte';

// Mock GSAP — we test DOM behaviour, not animation values.
vi.mock('gsap', () => ({
  gsap: {
    to: vi.fn(),
    fromTo: vi.fn(),
    set: vi.fn(),
  },
}));

// jsdom does not implement window.matchMedia; provide a stub.
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

describe('Button', () => {
  it('renders primary variant', () => {
    const { container } = render(Button, { variant: 'primary' });
    const btn = container.querySelector('button');
    expect(btn).not.toBeNull();
    expect(btn?.classList.contains('btn--primary')).toBe(true);
  });

  it('renders disabled state', () => {
    const { container } = render(Button, { disabled: true });
    const btn = container.querySelector('button');
    expect(btn).not.toBeNull();
    expect(btn?.disabled).toBe(true);
    expect(btn?.getAttribute('aria-disabled')).toBe('true');
  });

  it('calls onclick when clicked', async () => {
    const handler = vi.fn();
    const { container } = render(Button, { onclick: handler });
    const btn = container.querySelector('button');
    expect(btn).not.toBeNull();
    if (!btn) throw new Error('button element not found in container');
    await fireEvent.click(btn);
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('does not call onclick when disabled', async () => {
    const handler = vi.fn();
    const { container } = render(Button, { disabled: true, onclick: handler });
    const btn = container.querySelector('button');
    expect(btn).not.toBeNull();
    if (!btn) throw new Error('button element not found in container');
    // Disabled buttons do not fire click events in HTML; fireEvent bypasses
    // the browser guard, so we rely on our aria-disabled check in handleClick.
    await fireEvent.click(btn);
    expect(handler).not.toHaveBeenCalled();
  });

  it('shows loading spinner when loading=true', () => {
    const { container } = render(Button, { loading: true });
    const btn = container.querySelector('button');
    expect(btn).not.toBeNull();
    expect(btn?.getAttribute('aria-busy')).toBe('true');
    // Spinner element should be present
    const spinner = container.querySelector('.btn__spinner');
    expect(spinner).not.toBeNull();
    // Screen-reader label for loading state
    const srOnly = container.querySelector('.sr-only');
    expect(srOnly?.textContent).toBe('Loading…');
  });
});
