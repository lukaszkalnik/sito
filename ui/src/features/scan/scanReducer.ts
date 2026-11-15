import type { ScanCompleting, ScanErrorCode, ScanProgress, ScanSummary } from './api';

/** How far reading the EXIF left out by a quick first scan has got. */
export interface Completing {
  readonly done: number;
  readonly total: number;
}

/**
 * What of the open folder is known to be gone from disk: `none`, some
 * `photos` (moved, renamed or deleted outside Sito), or the whole `folder`.
 */
export type Missing = 'none' | 'photos' | 'folder';

type DoneState = {
  status: 'done';
  path: string;
  summary: ScanSummary;
  /** `null` when no EXIF is being read in the background. */
  completing: Completing | null;
  missing: Missing;
};

type SettledState =
  { status: 'idle' } | DoneState | { status: 'error'; path: string; code: ScanErrorCode };

export type ScanState =
  | SettledState
  | { status: 'picking'; previous: SettledState }
  | { status: 'scanning'; path: string; progress: ScanProgress };

/** Actions about refreshing the folder shown and files gone from disk. */
type RefreshAction =
  | { type: 'refreshStarted'; scanId: number }
  | { type: 'refreshed'; summary: ScanSummary }
  | { type: 'photosMissing'; scanId: number }
  | { type: 'folderMissing'; scanId: number };

export type ScanAction =
  | RefreshAction
  | { type: 'pickStarted' }
  | { type: 'pickCancelled' }
  | { type: 'scanStarted'; path: string }
  | { type: 'progress'; progress: ScanProgress }
  | { type: 'scanFinished'; summary: ScanSummary }
  | { type: 'scanFailed'; code: ScanErrorCode }
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
        ? {
            status: 'done',
            path: state.path,
            summary: action.summary,
            completing: null,
            missing: 'none',
          }
        : state;
    case 'scanFailed':
      return state.status === 'scanning'
        ? { status: 'error', path: state.path, code: action.code }
        : state;
    case 'completing':
      return updateShown(state, action.completing.scanId, (done) => ({
        ...done,
        completing: completingOf(action.completing),
      }));
    case 'refreshStarted':
    case 'refreshed':
    case 'photosMissing':
    case 'folderMissing':
      return refreshReducer(state, action);
  }
}

/** Refreshes of the folder shown and files found gone from disk. */
function refreshReducer(state: ScanState, action: RefreshAction): ScanState {
  switch (action.type) {
    case 'refreshStarted':
      // The missing photos may be found again; a folder that is gone stays until the end.
      return updateShown(state, action.scanId, (done) => ({
        ...done,
        summary: { ...done.summary, refreshing: true },
        missing: done.missing === 'photos' ? 'none' : done.missing,
      }));
    case 'refreshed':
      // A folder still gone is reported again right after.
      return updateShown(state, action.summary.scanId, (done) => ({
        ...done,
        summary: action.summary,
        missing: 'none',
      }));
    case 'photosMissing':
      // While refreshing they may well be found; a gone folder says more.
      return updateShown(state, action.scanId, (done) =>
        done.summary.refreshing || done.missing === 'folder'
          ? done
          : { ...done, missing: 'photos' },
      );
    case 'folderMissing':
      return updateShown(state, action.scanId, (done) => ({ ...done, missing: 'folder' }));
  }
}
