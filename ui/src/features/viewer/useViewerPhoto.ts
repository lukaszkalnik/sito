import { useEffect, useState } from 'react';

import { useTimelineRevision } from '../reject/useTimelineRevision';
import { getViewerPhoto, type ViewerPhoto, type ViewerRequest } from './api';
import { ViewerPageCache } from './viewerPageCache';

export type ViewerPhotoState =
  | { kind: 'loading' }
  | { kind: 'ready'; photo: ViewerPhoto }
  /** The photo is not on the timeline (yet), e.g. a restored one. */
  | { kind: 'missing' }
  /**
   * The photo shown left the timeline (a refresh found its file gone):
   * `afterReject` is the photo that took its place, as after a rejection.
   */
  | { kind: 'gone'; afterReject: number | null }
  /** The folder of the request's scan is no longer open in the main window. */
  | { kind: 'scanReplaced' }
  | { kind: 'error' };

/** What was loaded last, for which scan and photo. */
interface Loaded {
  scanId: number;
  id: number;
  state: ViewerPhotoState;
}

/**
 * The state for `request`, given what was `loaded` last. While another photo
 * of the same scan loads, the one shown before stays, so stepping never
 * flashes the spinner.
 */
function stateFor(loaded: Loaded | null, request: ViewerRequest): ViewerPhotoState {
  if (!loaded || loaded.scanId !== request.scanId) return { kind: 'loading' };
  if (loaded.id === request.id || loaded.state.kind === 'ready') return loaded.state;
  return { kind: 'loading' };
}

/**
 * The state of photo `id` of scan `scanId` that is not on the timeline:
 * `gone` if it was shown just `before` (it left the timeline), else
 * `missing` (not there yet).
 */
function absentState(before: Loaded | null, scanId: number, id: number): ViewerPhotoState {
  if (before?.scanId === scanId && before.id === id && before.state.kind === 'ready') {
    return { kind: 'gone', afterReject: before.state.photo.afterReject };
  }
  return { kind: 'missing' };
}

/** Starts loading the pages of `ids` (the neighbours of the shown photo). */
function prefetch(
  cache: ViewerPageCache,
  view: Omit<ViewerRequest, 'id'>,
  revision: number,
  ids: readonly (number | null)[],
): void {
  for (const id of ids) {
    // A failed prefetch is retried when the photo is shown.
    if (id !== null) cache.get({ ...view, id }, revision).catch(() => {});
  }
}

/**
 * Loads what the viewer shows for `request`, again whenever the timeline
 * changes, and prefetches the previous and next photo's pages.
 */
export function useViewerPhoto(request: ViewerRequest): ViewerPhotoState {
  const revision = useTimelineRevision();
  const [cache] = useState(() => new ViewerPageCache(getViewerPhoto));
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const { id, scanId, mode, gap } = request;

  useEffect(() => {
    let cancelled = false;
    const view = { scanId, mode, gap };
    const done = (state: ViewerPhotoState) => !cancelled && setLoaded({ scanId, id, state });
    const absent = () =>
      !cancelled && setLoaded((before) => ({ scanId, id, state: absentState(before, scanId, id) }));
    cache.get({ ...view, id }, revision).then(
      (answer) => {
        switch (answer.kind) {
          case 'missing':
            return absent();
          case 'scanReplaced':
            return done({ kind: 'scanReplaced' });
          case 'photo':
            done({ kind: 'ready', photo: answer.photo });
            return prefetch(cache, view, revision, [answer.photo.next, answer.photo.previous]);
        }
      },
      () => done({ kind: 'error' }),
    );
    return () => {
      cancelled = true;
    };
  }, [cache, id, scanId, mode, gap, revision]);

  return stateFor(loaded, request);
}
