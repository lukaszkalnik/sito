import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useMountedFor } from './useMountedFor';

describe('useMountedFor', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('turns true only after the delay', () => {
    const { result } = renderHook(() => useMountedFor(150));
    expect(result.current).toBe(false);

    act(() => vi.advanceTimersByTime(149));
    expect(result.current).toBe(false);

    act(() => vi.advanceTimersByTime(1));
    expect(result.current).toBe(true);
  });

  it('never fires after unmounting', () => {
    const { result, unmount } = renderHook(() => useMountedFor(150));

    unmount();
    act(() => vi.advanceTimersByTime(1000));

    expect(result.current).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });
});
