import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { onTimelineChanged } from '../reject/api';
import { getSeries, getTimeline, type Series } from './api';
import { useTimelineEntries } from './useTimelineEntries';

vi.mock('./api', async (importActual) => ({
  ...(await importActual<typeof import('./api')>()),
  getTimeline: vi.fn(),
  getSeries: vi.fn(),
}));
vi.mock('../reject/api', () => ({ onTimelineChanged: vi.fn() }));

const mockedTimeline = vi.mocked(getTimeline);
const mockedSeries = vi.mocked(getSeries);
/** Announces `timeline-changed` to the hook, like the backend after a rejection. */
let announceChange: () => void = () => {};

/** A promise with its resolve function, to answer requests in any order. */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

describe('useTimelineEntries', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mockedTimeline.mockResolvedValue([]);
    vi.mocked(onTimelineChanged).mockImplementation((handler) => {
      announceChange = handler;
      return Promise.resolve(() => {});
    });
  });

  it('reloads timeline and series when photos are rejected or restored', async () => {
    mockedSeries.mockResolvedValue([]);
    const scan = {};
    const { result } = renderHook(() => useTimelineEntries(scan, 10));
    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(mockedTimeline).toHaveBeenCalledTimes(1);

    mockedSeries.mockResolvedValue([{ start: 0, len: 2 }]);
    act(() => announceChange());

    await waitFor(() => expect(result.current).toMatchObject({ series: [{ start: 0, len: 2 }] }));
    expect(mockedTimeline).toHaveBeenCalledTimes(2);
  });

  it('keeps the previous series while new ones load and drops stale answers', async () => {
    const answers = new Map<number, ReturnType<typeof deferred<Series[]>>>();
    mockedSeries.mockImplementation((gap = 0) => {
      const answer = deferred<Series[]>();
      answers.set(gap, answer);
      return answer.promise;
    });
    const scan = {};
    const { result, rerender } = renderHook(({ gap }) => useTimelineEntries(scan, gap), {
      initialProps: { gap: 10 },
    });
    expect(result.current.status).toBe('loading');

    await act(async () => answers.get(10)?.resolve([{ start: 0, len: 10 }]));
    await waitFor(() => expect(result.current.status).toBe('ready'));

    rerender({ gap: 5 });
    rerender({ gap: 3 });
    // Still showing the series for 10 s while 5 s and 3 s are on their way.
    expect(result.current).toMatchObject({ series: [{ start: 0, len: 10 }] });

    await act(async () => answers.get(3)?.resolve([{ start: 0, len: 3 }]));
    await act(async () => answers.get(5)?.resolve([{ start: 0, len: 5 }]));

    expect(result.current).toMatchObject({ status: 'ready', series: [{ start: 0, len: 3 }] });
  });

  it('reports an error when the series cannot be loaded', async () => {
    mockedSeries.mockRejectedValue(new Error('boom'));
    const scan = {};
    const { result } = renderHook(() => useTimelineEntries(scan, 10));

    await waitFor(() => expect(result.current.status).toBe('error'));
  });
});
