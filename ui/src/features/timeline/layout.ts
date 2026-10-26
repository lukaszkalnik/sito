import type { TimelineEntry } from '../../api/scan';

/** Width of a tile in pixels. */
export const TILE_WIDTH = 160;
/** Height of a row of tiles in pixels, including the gap below it. */
export const TILE_ROW_HEIGHT = 128;
/** Height of a day header row in pixels. */
export const DAY_ROW_HEIGHT = 48;
/** Gap between tiles in pixels. */
export const TILE_GAP = 8;

/** One row of the virtualized timeline. */
export type TimelineRow =
  | { kind: 'day'; key: string; date: string }
  | { kind: 'tiles'; key: string; entries: TimelineEntry[] };

/** Number of tiles that fit next to each other in `width` pixels (at least 1). */
export function columnsFor(width: number): number {
  return Math.max(1, Math.floor((width + TILE_GAP) / (TILE_WIDTH + TILE_GAP)));
}

/** The calendar day (`YYYY-MM-DD`) of an entry's capture time. */
export function dayOf(entry: TimelineEntry): string {
  return entry.takenAt.slice(0, 10);
}

/**
 * Splits entries (already sorted newest first) into rows: a header for every
 * day, followed by that day's entries in rows of `columns` tiles.
 */
export function buildRows(entries: readonly TimelineEntry[], columns: number): TimelineRow[] {
  const rows: TimelineRow[] = [];
  let i = 0;
  while (i < entries.length) {
    const date = dayOf(entries[i] as TimelineEntry);
    rows.push({ kind: 'day', key: `day-${date}`, date });

    let end = i;
    while (end < entries.length && dayOf(entries[end] as TimelineEntry) === date) {
      end++;
    }
    for (let start = i; start < end; start += columns) {
      const chunk = entries.slice(start, Math.min(start + columns, end));
      rows.push({ kind: 'tiles', key: `tiles-${chunk[0]?.id ?? start}`, entries: chunk });
    }
    i = end;
  }
  return rows;
}

/** Height in pixels of a row. */
export function rowHeight(row: TimelineRow): number {
  return row.kind === 'day' ? DAY_ROW_HEIGHT : TILE_ROW_HEIGHT;
}
