import { useCallback, useEffect, useReducer, useRef } from 'react';

import { onScanRefreshed, pickFolder, ScanError, scanFolder } from '../../api/scan';
import { loadLastFolder, saveLastFolder } from './lastFolder';
import { initialScanState, scanReducer, type ScanState } from './scanReducer';

/**
 * Folder picking and scanning, as a state machine for the scan screen. The
 * folder opened last is opened again on start.
 */
export function useScan(): { state: ScanState; chooseAndScan: () => Promise<void> } {
  const [state, dispatch] = useReducer(scanReducer, initialScanState);

  const scan = useCallback(async (path: string) => {
    dispatch({ type: 'scanStarted', path });
    try {
      const summary = await scanFolder(path, (progress) =>
        dispatch({ type: 'progress', progress }),
      );
      dispatch({ type: 'scanFinished', summary });
      saveLastFolder(path);
    } catch (err) {
      dispatch({ type: 'scanFailed', code: err instanceof ScanError ? err.code : 'internal' });
    }
  }, []);

  const chooseAndScan = useCallback(async () => {
    dispatch({ type: 'pickStarted' });
    let path: string | null;
    try {
      path = await pickFolder();
    } catch {
      path = null;
    }
    if (path === null) {
      dispatch({ type: 'pickCancelled' });
      return;
    }
    await scan(path);
  }, [scan]);

  // Reopen the last folder once (StrictMode runs effects twice in development).
  const reopened = useRef(false);
  useEffect(() => {
    if (reopened.current) return;
    reopened.current = true;
    const last = loadLastFolder();
    if (last !== null) void scan(last);
  }, [scan]);

  // A folder shown from the cache gets its up-to-date summary when the
  // background rescan is done.
  useEffect(() => {
    const unlisten = onScanRefreshed((summary) => dispatch({ type: 'refreshed', summary })).catch(
      // Outside the app (e.g. a plain browser) there are no events.
      () => () => {},
    );
    return () => void unlisten.then((stop) => stop());
  }, []);

  return { state, chooseAndScan };
}
