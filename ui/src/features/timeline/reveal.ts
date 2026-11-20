import type { Undone } from '../reject/api';

/** Why the grid reveals a photo, which decides how long it is highlighted. */
export type RevealReason = 'restored' | 'viewerClosed';

/** A photo to scroll into view and highlight once the rows show it. */
export interface Reveal {
  readonly id: number;
  readonly reason: RevealReason;
  /** More photos highlighted along with it (e.g. all an undone edit changed); may include `id`. */
  readonly also?: readonly number[];
}

/**
 * How long a revealed photo stays highlighted, in milliseconds: a little
 * longer than its pulses in `TimelineGrid.css` (2 or 4 pulses of 1.25 s).
 */
export const HIGHLIGHT_MS: Readonly<Record<RevealReason, number>> = {
  restored: 2600,
  // The viewer often covered the grid, so the user needs longer to find the photo.
  viewerClosed: 5100,
};

/** How `highlighted` highlights the photo `id`: its reason, or `null` when it does not. */
export function highlightOf(highlighted: Reveal | null, id: number): RevealReason | null {
  if (highlighted === null) return null;
  const shown = highlighted.id === id || (highlighted.also?.includes(id) ?? false);
  return shown ? highlighted.reason : null;
}

/**
 * What the grid reveals after „Cofnij”: the photo that came back, or for an
 * undone edit the newest changed photo with all changed ones highlighted;
 * `null` when nothing is to be seen (a photo not on the timeline, or an
 * edit that changed no photo visibly).
 */
export function revealOfUndone(undone: Undone): Reveal | null {
  if (undone.kind === 'rejection') {
    return undone.id === null ? null : { id: undone.id, reason: 'restored' };
  }
  const [newest] = undone.changed;
  return newest === undefined ? null : { id: newest, reason: 'restored', also: undone.changed };
}
