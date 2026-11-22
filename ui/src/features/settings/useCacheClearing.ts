import { useState } from 'react';

import { CacheError, clearPhotoData, clearThumbnails, type CacheErrorCode } from './api';

/** What can be emptied. */
export type ClearTarget = 'thumbnails' | 'photoData';

/** Emptying a part of the cache: asked, running, failed, or nothing going on. */
export type ClearState =
  | { readonly kind: 'idle' }
  | { readonly kind: 'asking'; readonly target: ClearTarget }
  | { readonly kind: 'clearing'; readonly target: ClearTarget }
  | { readonly kind: 'failed'; readonly code: CacheErrorCode };

const CLEAR: Readonly<Record<ClearTarget, () => Promise<void>>> = {
  thumbnails: clearThumbnails,
  photoData: clearPhotoData,
};

/** Emptying thumbnails or photo data after a confirmation; `onCleared` runs after each. */
export function useCacheClearing(onCleared: () => void) {
  const [state, setState] = useState<ClearState>({ kind: 'idle' });

  const clear = async (target: ClearTarget) => {
    setState({ kind: 'clearing', target });
    try {
      await CLEAR[target]();
      setState({ kind: 'idle' });
    } catch (err) {
      setState({ kind: 'failed', code: err instanceof CacheError ? err.code : 'internal' });
    }
    onCleared();
  };

  return {
    state,
    ask: (target: ClearTarget) => setState({ kind: 'asking', target }),
    confirm: () => {
      if (state.kind === 'asking') void clear(state.target);
    },
    cancel: () => setState({ kind: 'idle' }),
  };
}
