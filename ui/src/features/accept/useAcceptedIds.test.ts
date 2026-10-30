import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { onTimelineChanged } from '../reject/api';
import { getAcceptedIds, onAcceptedChanged } from './api';
import { useAcceptedIds } from './useAcceptedIds';

vi.mock('./api', () => ({
  getAcceptedIds: vi.fn(),
  onAcceptedChanged: vi.fn(),
}));
vi.mock('../reject/api', () => ({
  onTimelineChanged: vi.fn(),
}));

/** Records the handler `listen` gets, so a test can send the event. */
function captureHandler(listen: typeof onAcceptedChanged) {
  const handlers: (() => void)[] = [];
  vi.mocked(listen).mockImplementation((handler) => {
    handlers.push(handler);
    return Promise.resolve(() => {});
  });
  return () => handlers.forEach((handler) => handler());
}

describe('useAcceptedIds', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getAcceptedIds).mockResolvedValue([2, 5]);
  });

  it('loads the accepted ids and reloads them when a mark changes', async () => {
    const acceptedChanged = captureHandler(onAcceptedChanged);
    captureHandler(onTimelineChanged);
    const { result } = renderHook(() => useAcceptedIds(1));
    await waitFor(() => expect(result.current).toEqual(new Set([2, 5])));

    vi.mocked(getAcceptedIds).mockResolvedValue([5]);
    act(() => acceptedChanged());

    await waitFor(() => expect(result.current).toEqual(new Set([5])));
  });

  it('reloads them when photos leave or return to the timeline', async () => {
    captureHandler(onAcceptedChanged);
    const timelineChanged = captureHandler(onTimelineChanged);
    const { result } = renderHook(() => useAcceptedIds(1));
    await waitFor(() => expect(result.current.size).toBe(2));

    vi.mocked(getAcceptedIds).mockResolvedValue([]);
    act(() => timelineChanged());

    await waitFor(() => expect(result.current.size).toBe(0));
  });

  it('never shows the ids of another scan', async () => {
    captureHandler(onAcceptedChanged);
    captureHandler(onTimelineChanged);
    const { result, rerender } = renderHook(({ scanId }) => useAcceptedIds(scanId), {
      initialProps: { scanId: 1 },
    });
    await waitFor(() => expect(result.current.size).toBe(2));

    vi.mocked(getAcceptedIds).mockReturnValue(new Promise(() => {}));
    rerender({ scanId: 2 });

    expect(result.current.size).toBe(0);
  });
});
