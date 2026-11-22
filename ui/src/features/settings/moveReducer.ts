import type { CacheErrorCode, CheckedFolder, MoveProgress } from './api';

/** Where the cache is to go: a folder the user chose, or back to the default one. */
export type MoveTarget =
  | { readonly kind: 'chosen'; readonly folder: string; readonly checked: CheckedFolder }
  | { readonly kind: 'default' };

/** The steps of moving the cache. */
export type MoveState =
  | { readonly kind: 'idle' }
  /** The folder picker is open, or the chosen folder is being checked. */
  | { readonly kind: 'choosing' }
  | { readonly kind: 'confirming'; readonly target: MoveTarget }
  /** Copying; `progress` is `null` until the first file is done. */
  | { readonly kind: 'moving'; readonly progress: MoveProgress | null }
  | { readonly kind: 'failed'; readonly code: CacheErrorCode };

export type MoveAction =
  | { readonly type: 'choose' }
  | { readonly type: 'chosen'; readonly target: MoveTarget }
  | { readonly type: 'notChosen' }
  | { readonly type: 'start' }
  | { readonly type: 'progress'; readonly progress: MoveProgress }
  | { readonly type: 'finished' }
  | { readonly type: 'failed'; readonly code: CacheErrorCode }
  | { readonly type: 'dismiss' };

export const initialMoveState: MoveState = { kind: 'idle' };

/** Whether the move can be started over (nothing in progress). */
function isSettled(state: MoveState): boolean {
  return state.kind === 'idle' || state.kind === 'failed';
}

/** The next step of the move. Actions that do not fit the current step change nothing. */
export function moveReducer(state: MoveState, action: MoveAction): MoveState {
  switch (action.type) {
    case 'choose':
      return isSettled(state) ? { kind: 'choosing' } : state;
    case 'chosen':
      return state.kind === 'choosing' || isSettled(state)
        ? { kind: 'confirming', target: action.target }
        : state;
    case 'notChosen':
      return state.kind === 'choosing' ? initialMoveState : state;
    case 'start':
      return state.kind === 'confirming' ? { kind: 'moving', progress: null } : state;
    case 'progress':
      return state.kind === 'moving' ? { kind: 'moving', progress: action.progress } : state;
    case 'finished':
      return initialMoveState;
    case 'failed':
      return { kind: 'failed', code: action.code };
    case 'dismiss':
      return state.kind === 'moving' ? state : initialMoveState;
  }
}
