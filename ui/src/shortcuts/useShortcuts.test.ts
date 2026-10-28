import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { useShortcuts } from './useShortcuts';

function press(key: string): KeyboardEvent {
  const event = new KeyboardEvent('keydown', { key, cancelable: true });
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
});
