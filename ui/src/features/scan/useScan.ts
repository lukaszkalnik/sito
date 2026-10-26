import { useCallback, useReducer } from 'react';

import { pickFolder, ScanError, scanFolder } from '../../api/scan';
import { initialScanState, scanReducer, type ScanState } from './scanReducer';

/** Folder picking and scanning, as a state machine for the scan screen. */
export function useScan(): { state: ScanState; chooseAndScan: () => Promise<void> } {
  const [state, dispatch] = useReducer(scanReducer, initialScanState);

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

    dispatch({ type: 'scanStarted', path });
    try {
      const summary = await scanFolder(path, (progress) =>
        dispatch({ type: 'progress', progress }),
      );
      dispatch({ type: 'scanFinished', summary });
    } catch (err) {
      dispatch({ type: 'scanFailed', code: err instanceof ScanError ? err.code : 'internal' });
    }
  }, []);

  return { state, chooseAndScan };
}
