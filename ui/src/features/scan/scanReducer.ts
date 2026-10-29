import type { ScanCompleting, ScanErrorCode, ScanProgress, ScanSummary } from './api';

/** How far reading the EXIF left out by a quick first scan has got. */
export interface Completing {
  readonly done: number;
  readonly total: number;
}

type DoneState = {
  status: 'done';
  path: string;
  summary: ScanSummary;
  /** `null` when no EXIF is being read in the background. */
  completing: Completing | null;
};

type SettledState =
  { status: 'idle' } | DoneState | { status: 'error'; path: string; code: ScanErrorCode };

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
  | { type: 'refreshed'; summary: ScanSummary }
  | { type: 'completing'; completing: ScanCompleting };

export const initialScanState: ScanState = { status: 'idle' };

/**
 * Applies `update` to the result of scan `scanId` if it is still the one
 * shown (also while another folder is being picked).
 */
function updateShown(
  state: ScanState,
  scanId: number,
  update: (done: DoneState) => DoneState,
): ScanState {
  if (state.status === 'done' && state.summary.scanId === scanId) {
    return update(state);
  }
  if (
    state.status === 'picking' &&
    state.previous.status === 'done' &&
    state.previous.summary.scanId === scanId
  ) {
    return { ...state, previous: update(state.previous) };
  }
  return state;
}

function completingOf({ done, total }: ScanCompleting): Completing | null {
  return done < total ? { done, total } : null;
}

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
        ? { status: 'done', path: state.path, summary: action.summary, completing: null }
        : state;
    case 'scanFailed':
      return state.status === 'scanning'
        ? { status: 'error', path: state.path, code: action.code }
        : state;
    case 'refreshed':
      return updateShown(state, action.summary.scanId, (done) => ({
        ...done,
        summary: action.summary,
      }));
    case 'completing':
      return updateShown(state, action.completing.scanId, (done) => ({
        ...done,
        completing: completingOf(action.completing),
      }));
  }
}
