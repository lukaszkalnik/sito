import type { ScanErrorCode, ScanProgress, ScanSummary } from '../../api/scan';

type SettledState =
  | { status: 'idle' }
  | { status: 'done'; path: string; summary: ScanSummary }
  | { status: 'error'; path: string; code: ScanErrorCode };

export type ScanState =
  | SettledState
  | { status: 'picking'; previous: SettledState }
  | { status: 'scanning'; path: string; progress: ScanProgress };

export type ScanAction =
  | { type: 'pickStarted' }
  | { type: 'pickCancelled' }
  | { type: 'scanStarted'; path: string }
  | { type: 'progress'; progress: ScanProgress }
  | { type: 'scanFinished'; summary: ScanSummary }
  | { type: 'scanFailed'; code: ScanErrorCode };

export const initialScanState: ScanState = { status: 'idle' };

export function scanReducer(state: ScanState, action: ScanAction): ScanState {
  switch (action.type) {
    case 'pickStarted':
      return state.status === 'scanning' || state.status === 'picking'
        ? state
        : { status: 'picking', previous: state };
    case 'pickCancelled':
      return state.status === 'picking' ? state.previous : state;
    case 'scanStarted':
      return {
        status: 'scanning',
        path: action.path,
        progress: { entriesSeen: 0, photosFound: 0 },
      };
    case 'progress':
      // Late progress messages after the scan settled are ignored.
      return state.status === 'scanning' ? { ...state, progress: action.progress } : state;
    case 'scanFinished':
      return state.status === 'scanning'
        ? { status: 'done', path: state.path, summary: action.summary }
        : state;
    case 'scanFailed':
      return state.status === 'scanning'
        ? { status: 'error', path: state.path, code: action.code }
        : state;
  }
}
