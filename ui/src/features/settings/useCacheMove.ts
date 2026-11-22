import { useReducer } from 'react';

import {
  CacheError,
  cancelCacheMove,
  checkCacheFolder,
  moveCache,
  pickCacheFolder,
  type CacheErrorCode,
  type CacheSettings,
} from './api';
import { initialMoveState, moveReducer, type MoveTarget } from './moveReducer';

function codeOf(err: unknown): CacheErrorCode {
  return err instanceof CacheError ? err.code : 'internal';
}

/**
 * Moving the cache: choosing a folder (checked before the confirmation),
 * confirming, the copy's progress and stopping it. `onMoved` gets the
 * settings once the cache is in its new place.
 */
export function useCacheMove(onMoved: (settings: CacheSettings) => void) {
  const [state, dispatch] = useReducer(moveReducer, initialMoveState);

  const choose = async () => {
    dispatch({ type: 'choose' });
    try {
      const folder = await pickCacheFolder();
      if (folder === null) {
        dispatch({ type: 'notChosen' });
        return;
      }
      const checked = await checkCacheFolder(folder);
      dispatch({ type: 'chosen', target: { kind: 'chosen', folder, checked } });
    } catch (err) {
      dispatch({ type: 'failed', code: codeOf(err) });
    }
  };
  const run = async (target: MoveTarget) => {
    dispatch({ type: 'start' });
    try {
      const folder = target.kind === 'chosen' ? target.folder : null;
      const moved = await moveCache(folder, (progress) => dispatch({ type: 'progress', progress }));
      dispatch({ type: 'finished' });
      onMoved(moved);
    } catch (err) {
      dispatch({ type: 'failed', code: codeOf(err) });
    }
  };

  return {
    state,
    choose: () => void choose(),
    restoreDefault: () => dispatch({ type: 'chosen', target: { kind: 'default' } }),
    confirm: () => {
      if (state.kind === 'confirming') void run(state.target);
    },
    /** Uses the default folder at once, without a confirmation (nothing moves). */
    useDefault: () => void run({ kind: 'default' }),
    stop: () => void cancelCacheMove().catch((err: unknown) => console.error(err)),
    dismiss: () => dispatch({ type: 'dismiss' }),
  };
}
