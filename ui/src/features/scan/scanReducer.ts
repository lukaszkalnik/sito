import type { ScanErrorCode, ScanProgress, ScanSummary } from './api';

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
  | { type: 'scanFailed'; code: ScanErrorCode }
  | { type: 'refreshed'; summary: ScanSummary };

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
        progress: { phase: 'searching', found: 0 },
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
    case 'refreshed':
      // Only for the scan still shown (also while picking another folder).
      if (state.status === 'done' && state.summary.scanId === action.summary.scanId) {
        return { ...state, summary: action.summary };
      }
      if (
        state.status === 'picking' &&
        state.previous.status === 'done' &&
        state.previous.summary.scanId === action.summary.scanId
      ) {
        return { ...state, previous: { ...state.previous, summary: action.summary } };
      }
      return state;
  }
}
