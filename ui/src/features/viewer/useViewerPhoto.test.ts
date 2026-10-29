import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { onTimelineChanged } from '../reject/api';
import type { TimelineEntry } from '../timeline/api';
import { getViewerPhoto, type ViewerPhoto, type ViewerRequest } from './api';
import { useViewerPhoto } from './useViewerPhoto';

vi.mock('./api', () => ({ getViewerPhoto: vi.fn() }));
vi.mock('../reject/api', () => ({ onTimelineChanged: vi.fn() }));

const mockedPage = vi.mocked(getViewerPhoto);
/** Announces `timeline-changed` to the hook, like the backend after a rejection. */
let announceChange: () => void = () => {};

function entry(id: number): TimelineEntry {
  return {
    id,
    name: `IMG_${id}.jpg`,
    relativeDir: '',
    relativePath: `IMG_${id}.jpg`,
    takenAt: '2024-06-01T12:00:00',
    timeSource: 'exif',
    format: 'jpeg',
  };
}

/** Photo `id` between `id - 1` and `id + 1`. */
function page(id: number): ViewerPhoto {
  return { entry: entry(id), previous: id - 1, next: id + 1, series: [], afterReject: null };
}

function request(id: number): ViewerRequest {
  return { id, scanId: 1, mode: 'all', gap: 10 };
}

const requestedIds = () => mockedPage.mock.calls.map(([r]) => r.id);

describe('useViewerPhoto', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mockedPage.mockImplementation((r) => Promise.resolve(page(r.id)));
    vi.mocked(onTimelineChanged).mockImplementation((handler) => {
      announceChange = handler;
      return Promise.resolve(() => {});
    });
  });

  it('loads the photo and prefetches its neighbours', async () => {
    const { result } = renderHook(() => useViewerPhoto(request(5)));

    await waitFor(() => expect(result.current).toEqual({ kind: 'ready', photo: page(5) }));
    await waitFor(() => expect(requestedIds()).toEqual([5, 6, 4]));
  });

  it('steps to a prefetched photo without asking again', async () => {
    const { result, rerender } = renderHook(({ id }) => useViewerPhoto(request(id)), {
      initialProps: { id: 5 },
    });
    await waitFor(() => expect(requestedIds()).toEqual([5, 6, 4]));

    rerender({ id: 6 });

    await waitFor(() => expect(result.current).toEqual({ kind: 'ready', photo: page(6) }));
    expect(requestedIds()).toEqual([5, 6, 4, 7]);
  });

  it('keeps the shown photo while the next one loads', async () => {
    const { result, rerender } = renderHook(({ id }) => useViewerPhoto(request(id)), {
      initialProps: { id: 5 },
    });
    await waitFor(() => expect(result.current.kind).toBe('ready'));
    mockedPage.mockReturnValue(new Promise(() => {}));

    rerender({ id: 9 });

    expect(result.current).toEqual({ kind: 'ready', photo: page(5) });
  });

  it('reports a photo that is not on the timeline, and finds it once the timeline changes', async () => {
    mockedPage.mockResolvedValueOnce(null);
    const { result } = renderHook(() => useViewerPhoto(request(5)));
    await waitFor(() => expect(result.current).toEqual({ kind: 'missing' }));

    act(() => announceChange());

    await waitFor(() => expect(result.current).toEqual({ kind: 'ready', photo: page(5) }));
  });

  it('reports a failed load', async () => {
    mockedPage.mockRejectedValue(new Error('gone'));
    const { result } = renderHook(() => useViewerPhoto(request(5)));

    await waitFor(() => expect(result.current).toEqual({ kind: 'error' }));
  });
});
