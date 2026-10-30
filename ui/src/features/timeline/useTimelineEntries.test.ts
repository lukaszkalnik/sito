import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { onTimelineChanged } from '../reject/api';
import { getSeries, getTimeline, type Grouping, type Series } from './api';
import { useTimelineEntries } from './useTimelineEntries';

vi.mock('./api', async (importActual) => ({
  ...(await importActual<typeof import('./api')>()),
  getTimeline: vi.fn(),
  getSeries: vi.fn(),
}));
vi.mock('../reject/api', () => ({ onTimelineChanged: vi.fn() }));

const mockedTimeline = vi.mocked(getTimeline);
const mockedSeries = vi.mocked(getSeries);

/** Series of an empty timeline, given as `[start, len]` ranges. */
function grouping(...ranges: [number, number][]): Grouping {
  const series: Series[] = ranges.map(([start, len]) => ({ start, len, label: { kind: 'plain' } }));
  return { order: [], series };
}
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
    mockedSeries.mockResolvedValue(grouping());
    const scan = {};
    const { result } = renderHook(() => useTimelineEntries(scan, 10));
    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(mockedTimeline).toHaveBeenCalledTimes(1);

    mockedSeries.mockResolvedValue(grouping([0, 2]));
    act(() => announceChange());

    await waitFor(() => expect(result.current).toMatchObject({ series: [{ start: 0, len: 2 }] }));
    expect(mockedTimeline).toHaveBeenCalledTimes(2);
  });

  it('keeps the previous series while new ones load and drops stale answers', async () => {
    const answers = new Map<number, ReturnType<typeof deferred<Grouping>>>();
    mockedSeries.mockImplementation((gap = 0) => {
      const answer = deferred<Grouping>();
      answers.set(gap, answer);
      return answer.promise;
    });
    const scan = {};
    const { result, rerender } = renderHook(({ gap }) => useTimelineEntries(scan, gap), {
      initialProps: { gap: 10 },
    });
    expect(result.current.status).toBe('loading');

    await act(async () => answers.get(10)?.resolve(grouping([0, 10])));
    await waitFor(() => expect(result.current.status).toBe('ready'));

    rerender({ gap: 5 });
    rerender({ gap: 3 });
    // Still showing the series for 10 s while 5 s and 3 s are on their way.
    expect(result.current).toMatchObject({ series: [{ start: 0, len: 10 }] });

    await act(async () => answers.get(3)?.resolve(grouping([0, 3])));
    await act(async () => answers.get(5)?.resolve(grouping([0, 5])));

    expect(result.current).toMatchObject({ status: 'ready', series: [{ start: 0, len: 3 }] });
  });

  it('arranges the entries in display order and loads the timeline once per change', async () => {
    const entry = (id: number) => ({
      id,
      name: `${id}.jpg`,
      relativeDir: '',
      relativePath: `${id}.jpg`,
      takenAt: '2024-06-01T12:00:00',
      timeSource: 'exif' as const,
      format: 'jpeg' as const,
    });
    mockedTimeline.mockResolvedValue([entry(0), entry(1), entry(2)]);
    mockedSeries.mockResolvedValue({ order: [0, 2, 1], series: [] });
    const scan = {};
    const { result, rerender } = renderHook(({ gap }) => useTimelineEntries(scan, gap), {
      initialProps: { gap: 10 },
    });

    await waitFor(() => expect(result.current.status).toBe('ready'));
    const ids = () =>
      result.current.status === 'ready' ? result.current.entries.map((e) => e.id) : [];
    expect(ids()).toEqual([0, 2, 1]);

    mockedSeries.mockResolvedValue({ order: [0, 1, 2], series: [] });
    rerender({ gap: 5 });
    await waitFor(() => expect(ids()).toEqual([0, 1, 2]));
    expect(mockedTimeline).toHaveBeenCalledTimes(1);
  });

  it('keeps the previous data when series do not match the timeline', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    mockedSeries.mockResolvedValue(grouping([0, 2]));
    const scan = {};
    const { result } = renderHook(() => useTimelineEntries(scan, 10));
    await waitFor(() => expect(result.current.status).toBe('ready'));

    mockedSeries.mockResolvedValue({ order: [0], series: [] });
    act(() => announceChange());

    await waitFor(() => expect(warn).toHaveBeenCalled());
    expect(result.current).toMatchObject({ status: 'ready', series: [{ start: 0, len: 2 }] });
    warn.mockRestore();
  });

  it('reports an error when the series cannot be loaded', async () => {
    mockedSeries.mockRejectedValue(new Error('boom'));
    const scan = {};
    const { result } = renderHook(() => useTimelineEntries(scan, 10));

    await waitFor(() => expect(result.current.status).toBe('error'));
  });
});
