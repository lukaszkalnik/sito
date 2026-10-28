import type { Series } from '../timeline/api';
import type { ViewMode } from './api';

/**
 * Positions (indices into the timeline) the viewer steps through, in timeline
 * order: every photo in `all` mode, only photos in series in `series` mode.
 */
export function navigationOrder(
  count: number,
  series: readonly Series[],
  mode: ViewMode,
): number[] {
  if (mode === 'all') return Array.from({ length: count }, (_, i) => i);
  return series.flatMap((s) => Array.from({ length: s.len }, (_, i) => s.start + i));
}

/**
 * The positions before and after `position` in `order` (sorted ascending), or
 * `null` at the ends. `position` itself need not be in `order`, e.g. a single
 * photo opened before switching to the series view.
 */
export function neighbours(
  order: readonly number[],
  position: number,
): { previous: number | null; next: number | null } {
  // First index in `order` whose value is >= position.
  let low = 0;
  let high = order.length;
  while (low < high) {
    const mid = (low + high) >> 1;
    if ((order[mid] ?? 0) < position) low = mid + 1;
    else high = mid;
  }
  const after = order[low] === position ? low + 1 : low;
  return { previous: order[low - 1] ?? null, next: order[after] ?? null };
}

/** The series containing timeline position `position`, if any. */
export function seriesOf(series: readonly Series[], position: number): Series | undefined {
  return series.find((s) => position >= s.start && position < s.start + s.len);
}

/**
 * The photo to show after rejecting the one at `position`: the next photo of
 * its series, else the previous one of its series, so you can keep sorting
 * the same series; only a single photo (or the last of a series) continues
 * with the next, else the previous, photo of the view. `null` if none is left.
 */
export function afterReject(
  count: number,
  series: readonly Series[],
  mode: ViewMode,
  position: number,
): number | null {
  const own = seriesOf(series, position);
  if (own && own.len > 1) {
    return position + 1 < own.start + own.len ? position + 1 : position - 1;
  }
  const { previous, next } = neighbours(navigationOrder(count, series, mode), position);
  return next ?? previous;
}
