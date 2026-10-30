/** Which photo the viewer window shows, as the main window follows it. */
export type FollowState =
  | { readonly kind: 'none' }
  | { readonly kind: 'open'; readonly scanId: number; readonly id: number }
  /** The viewer closed while showing `id`, which the grid has yet to reveal. */
  | { readonly kind: 'closed'; readonly scanId: number; readonly id: number };

/** What happens to the viewer, or to the reveal of its last photo. */
export type FollowAction =
  | { readonly kind: 'shown'; readonly scanId: number; readonly id: number }
  | { readonly kind: 'closed' }
  | { readonly kind: 'revealed' };

export const NOT_FOLLOWING: FollowState = { kind: 'none' };

/** The state after `action`. */
export function followReducer(state: FollowState, action: FollowAction): FollowState {
  switch (action.kind) {
    case 'shown':
      return { kind: 'open', scanId: action.scanId, id: action.id };
    case 'closed':
      // A repeated close keeps the photo still to reveal.
      return state.kind === 'none' ? state : { ...state, kind: 'closed' };
    case 'revealed':
      return state.kind === 'closed' ? NOT_FOLLOWING : state;
  }
}

/** The photo of scan `scanId` the open viewer shows, else `null`. */
export function followedId(state: FollowState, scanId: number): number | null {
  return state.kind === 'open' && state.scanId === scanId ? state.id : null;
}

/** The photo of scan `scanId` the viewer showed when it closed, until revealed; else `null`. */
export function lastShownId(state: FollowState, scanId: number): number | null {
  return state.kind === 'closed' && state.scanId === scanId ? state.id : null;
}
