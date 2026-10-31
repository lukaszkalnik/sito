import { useCallback, useEffect, useReducer, useRef } from 'react';

import { checkForUpdate, installUpdate, onCheckRequested, UpdateError } from './api';
import { initialUpdateState, updateReducer, type CheckTrigger } from './updateReducer';

function codeOf(error: unknown) {
  return error instanceof UpdateError ? error.code : 'internal';
}

/**
 * The update notice: checks for a newer version once when the main window
 * opens and again whenever the menu asks, and installs the update on demand.
 */
export function useUpdate() {
  const [state, dispatch] = useReducer(updateReducer, initialUpdateState);

  const check = useCallback((trigger: CheckTrigger) => {
    dispatch({ type: 'checkStarted', trigger });
    checkForUpdate()
      .then(({ currentVersion, update }) =>
        dispatch(
          update === null ? { type: 'notFound', currentVersion } : { type: 'found', info: update },
        ),
      )
      .catch((error: unknown) => {
        // Offline is normal: the quiet check at the start only logs it.
        console.error('Cannot check for an update', error);
        dispatch({ type: 'checkFailed', code: codeOf(error) });
      });
  }, []);

  // Once per app start; Strict Mode runs effects twice in development.
  const checkedAtStart = useRef(false);
  useEffect(() => {
    if (checkedAtStart.current) return;
    checkedAtStart.current = true;
    check('start');
  }, [check]);

  useEffect(() => {
    const unlisten = onCheckRequested(() => check('menu')).catch(
      // Outside the app (e.g. a plain browser) there are no events.
      () => () => {},
    );
    return () => void unlisten.then((stop) => stop());
  }, [check]);

  const install = useCallback(() => {
    dispatch({ type: 'installStarted' });
    installUpdate((progress) => dispatch({ type: 'progress', progress })).catch(
      (error: unknown) => {
        console.error('Cannot install the update', error);
        dispatch({ type: 'installFailed', code: codeOf(error) });
      },
    );
  }, []);

  const dismiss = useCallback(() => dispatch({ type: 'dismissed' }), []);

  return { state, install, dismiss };
}
