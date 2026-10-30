/** Why the grid reveals a photo, which decides how long it is highlighted. */
export type RevealReason = 'restored' | 'viewerClosed';

/** A photo to scroll into view and highlight once the rows show it. */
export interface Reveal {
  readonly id: number;
  readonly reason: RevealReason;
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
