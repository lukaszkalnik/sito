import { useCallback, useEffect, useReducer } from 'react';

import { onViewerClosed, onViewerShown } from '../viewer/api';
import { followedId, followReducer, lastShownId, NOT_FOLLOWING } from './followViewer';

/**
 * Follows the viewer window: `followed` is the photo of scan `scanId` it
 * shows while open, `lastShown` the one it showed when it closed, until
 * `revealed` is called.
 */
export function useFollowViewer(scanId: number) {
  const [state, dispatch] = useReducer(followReducer, NOT_FOLLOWING);

  useEffect(() => {
    const unlisteners = [
      onViewerShown((shown) => dispatch({ kind: 'shown', ...shown })),
      onViewerClosed(() => dispatch({ kind: 'closed' })),
    ];
    return () => {
      for (const unlisten of unlisteners) {
        void unlisten
          .then((stop) => stop())
          .catch((err: unknown) => console.error('Cannot follow the viewer', err));
      }
    };
  }, []);

  const revealed = useCallback(() => dispatch({ kind: 'revealed' }), []);
  return {
    followed: followedId(state, scanId),
    lastShown: lastShownId(state, scanId),
    revealed,
  };
}
