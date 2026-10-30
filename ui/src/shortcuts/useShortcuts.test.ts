import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { useShortcuts } from './useShortcuts';

function press(key: string, repeat = false): KeyboardEvent {
  const event = new KeyboardEvent('keydown', { key, repeat, cancelable: true });
  window.dispatchEvent(event);
  return event;
}

describe('useShortcuts', () => {
  it('runs the handler of the pressed key and claims the key', () => {
    const reject = vi.fn();
    renderHook(() => useShortcuts({ reject }));

    expect(press('Delete').defaultPrevented).toBe(true);
    expect(reject).toHaveBeenCalledOnce();
  });

  it('leaves keys of unavailable or unknown actions to the browser', () => {
    renderHook(() => useShortcuts({ previous: null }));

    expect(press('ArrowLeft').defaultPrevented).toBe(false);
    expect(press('a').defaultPrevented).toBe(false);
  });

  it('uses the latest handlers and stops listening on unmount', () => {
    const first = vi.fn();
    const second = vi.fn();
    const { rerender, unmount } = renderHook(({ close }) => useShortcuts({ close }), {
      initialProps: { close: first },
    });
    rerender({ close: second });
    press('Escape');
    unmount();
    press('Escape');

    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledOnce();
  });

  it('rejects once per press, not again while Delete is held', () => {
    const reject = vi.fn();
    renderHook(() => useShortcuts({ reject }));

    press('Delete');
    const repeated = press('Delete', true);
    press('Delete', true);

    expect(reject).toHaveBeenCalledOnce();
    // The held key is still claimed, so the browser does nothing with it either.
    expect(repeated.defaultPrevented).toBe(true);
  });

  it('keeps stepping while an arrow key is held', () => {
    const next = vi.fn();
    renderHook(() => useShortcuts({ next }));

    press('ArrowRight');
    press('ArrowRight', true);
    press('ArrowRight', true);

    expect(next).toHaveBeenCalledTimes(3);
  });
});
