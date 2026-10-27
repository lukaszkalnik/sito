import type { Series, TimelineEntry } from '../../api/scan';

/** Width of a tile in pixels. */
export const TILE_WIDTH = 160;
/** Height of a row of tiles in pixels, including the gap below it. */
export const TILE_ROW_HEIGHT = 128;
/** Height of a day header row in pixels. */
export const DAY_ROW_HEIGHT = 48;
/** Height of a series header row in pixels. */
export const SERIES_ROW_HEIGHT = 32;
/** Extra space below the last tile row of a series, separating it from what follows. */
export const SERIES_END_SPACE = 8;
/** Inner padding on both sides of a series. */
export const SERIES_PADDING = 8;
/** Gap between tiles in pixels. */
export const TILE_GAP = 8;

/** Which photos the timeline shows: only series, or all photos with series marked. */
export type TimelineMode = 'series' | 'all';

/** One row of the virtualized timeline. */
export type TimelineRow =
  | { kind: 'day'; key: string; date: string }
  | { kind: 'series'; key: string; entries: TimelineEntry[] }
  | { kind: 'tiles'; key: string; entries: TimelineEntry[]; inSeries: boolean; seriesEnd: boolean };

/**
 * Number of tiles that fit next to each other in `width` pixels (at least 1).
 * Leaves room for the padding of a series, so all rows have the same columns.
 */
export function columnsFor(width: number): number {
  const usable = width - 2 * SERIES_PADDING;
  return Math.max(1, Math.floor((usable + TILE_GAP) / (TILE_WIDTH + TILE_GAP)));
}

/** The calendar day (`YYYY-MM-DD`) of an entry's capture time. */
export function dayOf(entry: TimelineEntry): string {
  return entry.takenAt.slice(0, 10);
}

/**
 * Splits entries (sorted newest first) into rows: a header for every day,
 * then that day's photos in rows of `columns` tiles. Every series starts on a
 * new row under its own header; it belongs to the day of its newest photo, so
 * a series crossing midnight is never split. In `series` mode photos outside
 * series (and days without series) are left out.
 */
export function buildRows(
  entries: readonly TimelineEntry[],
  series: readonly Series[],
  columns: number,
  mode: TimelineMode,
): TimelineRow[] {
  const seriesAt = new Map(series.map((s) => [s.start, s]));
  const rows: TimelineRow[] = [];
  let currentDay: string | null = null;
  let singles: TimelineEntry[] = [];

  const pushTiles = (photos: TimelineEntry[], inSeries: boolean) => {
    for (let start = 0; start < photos.length; start += columns) {
      const chunk = photos.slice(start, start + columns);
      rows.push({
        kind: 'tiles',
        key: `tiles-${chunk[0]?.id ?? start}`,
        entries: chunk,
        inSeries,
        seriesEnd: inSeries && start + columns >= photos.length,
      });
    }
  };
  const flushSingles = () => {
    pushTiles(singles, false);
    singles = [];
  };
  const enterDay = (entry: TimelineEntry) => {
    const date = dayOf(entry);
    if (date === currentDay) return;
    flushSingles();
    rows.push({ kind: 'day', key: `day-${date}`, date });
    currentDay = date;
  };

  let i = 0;
  while (i < entries.length) {
    const entry = entries[i] as TimelineEntry;
    const s = seriesAt.get(i);
    if (s) {
      enterDay(entry);
      flushSingles();
      const members = entries.slice(i, i + s.len);
      rows.push({ kind: 'series', key: `series-${entry.id}`, entries: members });
      pushTiles(members, true);
      i += s.len;
    } else {
      if (mode === 'all') {
        enterDay(entry);
        singles.push(entry);
      }
      i++;
    }
  }
  flushSingles();
  return rows;
}

/** Height in pixels of a row. */
export function rowHeight(row: TimelineRow): number {
  switch (row.kind) {
    case 'day':
      return DAY_ROW_HEIGHT;
    case 'series':
      return SERIES_ROW_HEIGHT;
    case 'tiles':
      return TILE_ROW_HEIGHT + (row.seriesEnd ? SERIES_END_SPACE : 0);
  }
}
