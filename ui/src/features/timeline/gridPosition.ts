import type { TimelineEntry } from './api';
import { TILE_GAP, TILE_ROW_HEIGHT, tilePositions, type TimelineRow } from './layout';

/**
 * Where the grid was scrolled to, independent of its rows: the photo at the
 * top of the view (by file path, since photo ids change with every scan),
 * its capture time for finding a neighbour when it is gone, and how many
 * pixels of its tile were scrolled past (negative when it starts below the
 * top of the view).
 */
export interface GridPosition {
  /** File path relative to the scanned folder. */
  readonly path: string;
  /** Capture time, ISO 8601 without offset. */
  readonly takenAt: string;
  /** Pixels between the top of the photo's tile and the top of the view. */
  readonly offset: number;
}

/**
 * The position of the grid scrolled to `scrollTop` in `rows`: its topmost
 * (partly) visible photo. `null` at the very top, so a grid left there opens
 * at the top again, and when no photo is visible.
 */
export function topPosition(rows: readonly TimelineRow[], scrollTop: number): GridPosition | null {
  if (scrollTop <= 0) return null;
  for (const { entry, top } of tilePositions(rows)) {
    // Only the tiles count as visible, not a header or the space below them.
    if (top + TILE_ROW_HEIGHT - TILE_GAP <= scrollTop) continue;
    return { path: entry.relativePath, takenAt: entry.takenAt, offset: scrollTop - top };
  }
  return null;
}

/**
 * Scroll position putting `position`'s photo back at the same place in the
 * view. If the rows no longer show it (rejected, moved, or hidden by the
 * mode), the first photo not newer than it takes its place, else the last
 * photo. `null` when the rows show no photo.
 */
export function restoredScrollTop(
  rows: readonly TimelineRow[],
  position: GridPosition,
): number | null {
  let fallback: number | null = null;
  let last: number | null = null;
  for (const { entry, top } of tilePositions(rows)) {
    if (entry.relativePath === position.path) return scrollTopFor(top, position);
    if (fallback === null && notNewer(entry, position.takenAt)) fallback = top;
    last = top;
  }
  const top = fallback ?? last;
  return top === null ? null : scrollTopFor(top, position);
}

function scrollTopFor(tileTop: number, position: GridPosition): number {
  return Math.max(0, tileTop + position.offset);
}

/** Whether `entry` was taken at or before `takenAt`; both are ISO 8601, so they compare as text. */
function notNewer(entry: TimelineEntry, takenAt: string): boolean {
  return entry.takenAt <= takenAt;
}
