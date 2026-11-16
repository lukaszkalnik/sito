import { useCallback, useEffect, useReducer, useRef } from 'react';

import {
  onFolderMissing,
  onPhotosMissing,
  onScanCompleting,
  onScanRefreshed,
  onScanRefreshing,
  pickFolder,
  refreshFolder,
  ScanError,
  scanFolder,
} from './api';
import { loadLastFolder, saveLastFolder } from './lastFolder';
import { initialScanState, scanReducer, type ScanAction, type ScanState } from './scanReducer';

/** Dispatches `toAction(scanId)` for every scan event that `listen` subscribes to. */
function useScanEvent(
  listen: (handler: (scanId: number) => void) => Promise<() => void>,
  dispatch: (action: ScanAction) => void,
  toAction: (scanId: number) => ScanAction,
) {
  useEffect(() => {
    // Outside the app (e.g. a plain browser) there are no events.
    const unlisten = listen((scanId) => dispatch(toAction(scanId))).catch(() => () => {});
    return () => void unlisten.then((stop) => stop());
  }, [listen, dispatch, toAction]);
}

const refreshStarted = (scanId: number): ScanAction => ({ type: 'refreshStarted', scanId });
const photosMissing = (scanId: number): ScanAction => ({ type: 'photosMissing', scanId });
const folderMissing = (scanId: number): ScanAction => ({ type: 'folderMissing', scanId });

/** Background work on the folder shown, and files found gone from disk. */
function useScanEvents(dispatch: (action: ScanAction) => void) {
  // A folder shown from the cache gets its up-to-date summary when the
  // background rescan is done.
  useEffect(() => {
    const unlisten = onScanRefreshed((summary) => dispatch({ type: 'refreshed', summary })).catch(
      // Outside the app (e.g. a plain browser) there are no events.
      () => () => {},
    );
    return () => void unlisten.then((stop) => stop());
  }, [dispatch]);

  // A quick first scan reads the EXIF of its photos afterwards.
  useEffect(() => {
    const unlisten = onScanCompleting((completing) =>
      dispatch({ type: 'completing', completing }),
    ).catch(() => () => {});
    return () => void unlisten.then((stop) => stop());
  }, [dispatch]);

  useScanEvent(onScanRefreshing, dispatch, refreshStarted);
  useScanEvent(onPhotosMissing, dispatch, photosMissing);
  useScanEvent(onFolderMissing, dispatch, folderMissing);
}

/**
 * Folder picking and scanning, as a state machine for the scan screen. The
 * folder opened last is opened again on start; `refresh` rescans the open
 * folder in the background.
 */
export function useScan(): {
  state: ScanState;
  chooseAndScan: () => Promise<void>;
  refresh: () => void;
} {
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
      const code = err instanceof ScanError ? err.code : 'internal';
      // A later opening (by a reloaded page) shows its own result.
      if (code === 'superseded') return;
      dispatch({ type: 'scanFailed', code });
    }
  }, []);

  const chooseAndScan = useCallback(async () => {
    dispatch({ type: 'pickStarted' });
    let path: string | null;
    try {
      path = await pickFolder();
    } catch (err) {
      console.error('Cannot open the folder picker', err);
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

  useScanEvents(dispatch);

  // The refresh announces itself with `scan-refreshing`, also when started in the viewer.
  const refresh = useCallback(() => {
    refreshFolder().catch((err: unknown) => console.error('Cannot refresh the folder', err));
  }, []);

  return { state, chooseAndScan, refresh };
}
