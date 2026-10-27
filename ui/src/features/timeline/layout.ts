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

/** Space between two series blocks sharing a row. */
export const BLOCK_GAP = TILE_GAP;
/** Height of a row of series blocks: header, one row of tiles and the block's bottom padding. */
export const BLOCK_ROW_HEIGHT = SERIES_ROW_HEIGHT + TILE_ROW_HEIGHT + SERIES_END_SPACE;

/** Photos side by side in a row of blocks: one short series, or single photos. */
export interface RowGroup {
  entries: TimelineEntry[];
  /** A series (a tinted block with a header), or consecutive single photos. */
  series: boolean;
}

/** One row of the virtualized timeline. */
export type TimelineRow =
  | { kind: 'day'; key: string; date: string }
  | { kind: 'series'; key: string; entries: TimelineEntry[] }
  | { kind: 'tiles'; key: string; entries: TimelineEntry[]; inSeries: boolean; seriesEnd: boolean }
  /** Short series, each in its own block with a header, mixed with single photos. */
  | { kind: 'blocks'; key: string; groups: RowGroup[] };

/** Width in pixels of a series block of `len` tiles, padding included. */
export function blockWidth(len: number): number {
  return len * TILE_WIDTH + (len - 1) * TILE_GAP + 2 * SERIES_PADDING;
}

/** The photos of a row, with the distance from the row's top to its tiles. */
export function tilesOf(row: TimelineRow): { entries: TimelineEntry[]; offset: number } {
  switch (row.kind) {
    case 'tiles':
      return { entries: row.entries, offset: 0 };
    case 'blocks':
      return { entries: row.groups.flatMap((g) => g.entries), offset: SERIES_ROW_HEIGHT };
    default:
      return { entries: [], offset: 0 };
  }
}

/** Whether the photos come from more than one folder, so tiles need to name theirs. */
export function spansSeveralFolders(entries: readonly TimelineEntry[]): boolean {
  return entries.some((entry) => entry.relativeDir !== entries[0]?.relativeDir);
}

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
 * then that day's photos flowing into rows `width` pixels wide. A series that
 * fits on one row becomes a block with its own header and flows along with
 * single photos and other blocks; a longer series starts on a new row under
 * its own header. A row of single photos only stays a plain, lower row of
 * `columns` tiles. A series belongs to the day of its newest photo, so one
 * crossing midnight is never split. In `series` mode photos outside series
 * (and days without series) are left out.
 */
export function buildRows(
  entries: readonly TimelineEntry[],
  series: readonly Series[],
  columns: number,
  mode: TimelineMode,
  width: number = blockWidth(columns),
): TimelineRow[] {
  const seriesAt = new Map(series.map((s) => [s.start, s]));
  const rows: TimelineRow[] = [];
  let currentDay: string | null = null;
  // The row being filled and its used width, until it is closed.
  let open: { groups: RowGroup[]; used: number } | null = null;

  const closeRow = () => {
    if (!open) return;
    const { groups } = open;
    const first = groups[0]?.entries[0]?.id;
    if (groups.some((g) => g.series)) {
      rows.push({ kind: 'blocks', key: `blocks-${first}`, groups });
    } else {
      const photos = groups.flatMap((g) => g.entries);
      rows.push({
        kind: 'tiles',
        key: `tiles-${first}`,
        entries: photos,
        inSeries: false,
        seriesEnd: false,
      });
    }
    open = null;
  };

  const placeSeries = (members: TimelineEntry[]) => {
    const needed = blockWidth(members.length);
    if (open && open.used + BLOCK_GAP + needed <= width) {
      open.groups.push({ entries: members, series: true });
      open.used += BLOCK_GAP + needed;
      return;
    }
    closeRow();
    open = { groups: [{ entries: members, series: true }], used: needed };
  };

  const placeSingle = (photo: TimelineEntry) => {
    const last = open?.groups[open.groups.length - 1];
    // Next to other single photos a tile costs one column; after a block it
    // starts its own group, with its own padding.
    const needed = last && !last.series ? TILE_WIDTH + TILE_GAP : BLOCK_GAP + blockWidth(1);
    if (open && last && open.used + needed <= width) {
      if (last.series) open.groups.push({ entries: [photo], series: false });
      else last.entries.push(photo);
      open.used += needed;
      return;
    }
    closeRow();
    open = { groups: [{ entries: [photo], series: false }], used: blockWidth(1) };
  };

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
  const enterDay = (entry: TimelineEntry) => {
    const date = dayOf(entry);
    if (date === currentDay) return;
    closeRow();
    rows.push({ kind: 'day', key: `day-${date}`, date });
    currentDay = date;
  };

  let i = 0;
  while (i < entries.length) {
    const entry = entries[i] as TimelineEntry;
    const s = seriesAt.get(i);
    if (s) {
      enterDay(entry);
      const members = entries.slice(i, i + s.len);
      if (members.length <= columns) {
        placeSeries(members);
      } else {
        closeRow();
        rows.push({ kind: 'series', key: `series-${entry.id}`, entries: members });
        pushTiles(members, true);
      }
      i += s.len;
    } else {
      if (mode === 'all') {
        enterDay(entry);
        placeSingle(entry);
      }
      i++;
    }
  }
  closeRow();
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
    case 'blocks':
      return BLOCK_ROW_HEIGHT;
  }
}

/** Total height in pixels of all rows. */
export function totalHeight(rows: readonly TimelineRow[]): number {
  return rows.reduce((sum, row) => sum + rowHeight(row), 0);
}

/**
 * Scroll position for `newRows` that keeps the photos seen at `scrollTop` in
 * `oldRows` in place, for regrouped series or a switched mode. The anchor is
 * the topmost visible photo that `newRows` still shows (singles disappear in
 * `series` mode); it stays at the same distance from the top of the view.
 */
export function anchoredScrollTop(
  oldRows: readonly TimelineRow[],
  newRows: readonly TimelineRow[],
  scrollTop: number,
): number {
  if (scrollTop <= 0) return 0;

  // Top of each photo's tile in the new layout.
  const newTops = new Map<number, number>();
  let start = 0;
  for (const row of newRows) {
    const { entries, offset } = tilesOf(row);
    for (const entry of entries) newTops.set(entry.id, start + offset);
    start += rowHeight(row);
  }
  const newTotal = start;

  start = 0;
  for (const row of oldRows) {
    const { entries, offset } = tilesOf(row);
    const top = start + offset;
    // Only the tiles count as visible, not a header or the space below them.
    if (entries.length > 0 && top + TILE_ROW_HEIGHT - TILE_GAP > scrollTop) {
      for (const entry of entries) {
        const newTop = newTops.get(entry.id);
        if (newTop !== undefined) return Math.max(0, newTop - (top - scrollTop));
      }
    }
    start += rowHeight(row);
  }
  return newTotal;
}

/**
 * Scroll position that shows the photo `id` in a view `viewHeight` pixels
 * tall: the current `scrollTop` if the photo is already fully visible, else
 * one with the photo in the middle. `null` if the rows do not show the photo.
 */
export function revealScrollTop(
  rows: readonly TimelineRow[],
  id: number,
  scrollTop: number,
  viewHeight: number,
): number | null {
  const tileHeight = TILE_ROW_HEIGHT - TILE_GAP;
  let start = 0;
  for (const row of rows) {
    const { entries, offset } = tilesOf(row);
    if (entries.some((entry) => entry.id === id)) {
      const top = start + offset;
      if (top >= scrollTop && top + tileHeight <= scrollTop + viewHeight) return scrollTop;
      return Math.max(0, Math.round(top - (viewHeight - tileHeight) / 2));
    }
    start += rowHeight(row);
  }
  return null;
}
