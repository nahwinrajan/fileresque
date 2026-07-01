import { fireEvent, render } from '@testing-library/svelte';
import { createRawSnippet, tick } from 'svelte';
import { beforeAll, describe, expect, it, vi } from 'vitest';
import Modal from './Modal.svelte';

// Test DOM behaviour, not animation values.
vi.mock('gsap', () => ({
  gsap: { to: vi.fn(), fromTo: vi.fn(), set: vi.fn() },
}));

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

// A body snippet with one extra focusable, so the dialog holds ≥2 tab stops
// (the close button + this one) and wrap-around can be exercised.
const bodyWithButton = createRawSnippet(() => ({
  render: () => '<button type="button" data-testid="inner">inner</button>',
}));

describe('Modal focus management', () => {
  it('wraps focus forward from the last focusable to the first', async () => {
    const { container } = render(Modal, { open: true, title: 'T', children: bodyWithButton });
    await tick();

    const focusables = container.querySelectorAll<HTMLElement>('button');
    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    expect(focusables.length).toBeGreaterThanOrEqual(2);

    last.focus();
    await fireEvent.keyDown(window, { key: 'Tab' });
    expect(document.activeElement).toBe(first);
  });

  it('wraps focus backward from the first focusable to the last', async () => {
    const { container } = render(Modal, { open: true, title: 'T', children: bodyWithButton });
    await tick();

    const focusables = container.querySelectorAll<HTMLElement>('button');
    const first = focusables[0];
    const last = focusables[focusables.length - 1];

    first.focus();
    await fireEvent.keyDown(window, { key: 'Tab', shiftKey: true });
    expect(document.activeElement).toBe(last);
  });

  it('restores focus to the triggering element on close', async () => {
    const trigger = document.createElement('button');
    document.body.appendChild(trigger);
    trigger.focus();
    expect(document.activeElement).toBe(trigger);

    const { rerender } = render(Modal, { open: true, title: 'T', children: bodyWithButton });
    await tick();
    // Focus moved into the dialog while open.
    expect(document.activeElement).not.toBe(trigger);

    await rerender({ open: false, title: 'T', children: bodyWithButton });
    await tick();
    expect(document.activeElement).toBe(trigger);

    trigger.remove();
  });

  it('closes on Escape', async () => {
    const onclose = vi.fn();
    render(Modal, { open: true, title: 'T', onclose, children: bodyWithButton });
    await tick();
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(onclose).toHaveBeenCalledTimes(1);
  });
});
