import type { Series, TimelineEntry } from './api';

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
    case 'day':
    case 'series':
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
  const builder = new RowBuilder(columns, width);
  let i = 0;
  while (i < entries.length) {
    const entry = entries[i] as TimelineEntry;
    const s = seriesAt.get(i);
    if (s) {
      builder.addSeries(entries.slice(i, i + s.len));
      i += s.len;
    } else {
      if (mode === 'all') builder.addSingle(entry);
      i++;
    }
  }
  return builder.finish();
}

/** Collects the rows of {@link buildRows}, filling one open row at a time. */
class RowBuilder {
  private readonly rows: TimelineRow[] = [];
  private currentDay: string | null = null;
  /** The row being filled and its used width, until it is closed. */
  private open: { groups: RowGroup[]; used: number } | null = null;

  private readonly columns: number;
  private readonly width: number;

  constructor(columns: number, width: number) {
    this.columns = columns;
    this.width = width;
  }

  /** Adds a series: a block in the open row if it fits on one row, else its own rows. */
  addSeries(members: TimelineEntry[]): void {
    const first = members[0];
    if (!first) return;
    this.enterDay(first);
    if (members.length <= this.columns) {
      this.placeBlock(members);
      return;
    }
    this.closeRow();
    this.rows.push({ kind: 'series', key: `series-${first.id}`, entries: members });
    this.pushSeriesTiles(members);
  }

  /** Adds a photo outside any series. */
  addSingle(photo: TimelineEntry): void {
    this.enterDay(photo);
    const last = this.open?.groups.at(-1);
    // Next to other single photos a tile costs one column; after a block it
    // starts its own group, with its own padding.
    const needed = last && !last.series ? TILE_WIDTH + TILE_GAP : BLOCK_GAP + blockWidth(1);
    if (!this.open || !last || this.open.used + needed > this.width) {
      this.closeRow();
      this.open = { groups: [{ entries: [photo], series: false }], used: blockWidth(1) };
      return;
    }
    if (last.series) this.open.groups.push({ entries: [photo], series: false });
    else last.entries.push(photo);
    this.open.used += needed;
  }

  /** Closes the open row and returns all rows. */
  finish(): TimelineRow[] {
    this.closeRow();
    return this.rows;
  }

  private placeBlock(members: TimelineEntry[]): void {
    const needed = blockWidth(members.length);
    const group = { entries: members, series: true };
    if (this.open && this.open.used + BLOCK_GAP + needed <= this.width) {
      this.open.groups.push(group);
      this.open.used += BLOCK_GAP + needed;
      return;
    }
    this.closeRow();
    this.open = { groups: [group], used: needed };
  }

  private pushSeriesTiles(photos: TimelineEntry[]): void {
    for (let start = 0; start < photos.length; start += this.columns) {
      const chunk = photos.slice(start, start + this.columns);
      this.rows.push({
        kind: 'tiles',
        key: `tiles-${chunk[0]?.id ?? start}`,
        entries: chunk,
        inSeries: true,
        seriesEnd: start + this.columns >= photos.length,
      });
    }
  }

  private enterDay(entry: TimelineEntry): void {
    const date = dayOf(entry);
    if (date === this.currentDay) return;
    this.closeRow();
    this.rows.push({ kind: 'day', key: `day-${date}`, date });
    this.currentDay = date;
  }

  private closeRow(): void {
    if (!this.open) return;
    const { groups } = this.open;
    const key = groups[0]?.entries[0]?.id;
    if (groups.some((g) => g.series)) {
      this.rows.push({ kind: 'blocks', key: `blocks-${key}`, groups });
    } else {
      const entries = groups.flatMap((g) => g.entries);
      this.rows.push({
        kind: 'tiles',
        key: `tiles-${key}`,
        entries,
        inSeries: false,
        seriesEnd: false,
      });
    }
    this.open = null;
  }
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

  const newTops = tileTops(newRows);
  let start = 0;
  for (const row of oldRows) {
    const { entries, offset } = tilesOf(row);
    const top = start + offset;
    start += rowHeight(row);
    // Only the tiles count as visible, not a header or the space below them.
    if (entries.length === 0 || top + TILE_ROW_HEIGHT - TILE_GAP <= scrollTop) continue;
    const newTop = entries.map((entry) => newTops.get(entry.id)).find((t) => t !== undefined);
    if (newTop !== undefined) return Math.max(0, newTop - (top - scrollTop));
  }
  return totalHeight(newRows);
}

/** Top of each photo's tile, by photo id. */
function tileTops(rows: readonly TimelineRow[]): Map<number, number> {
  const tops = new Map<number, number>();
  let start = 0;
  for (const row of rows) {
    const { entries, offset } = tilesOf(row);
    for (const entry of entries) tops.set(entry.id, start + offset);
    start += rowHeight(row);
  }
  return tops;
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
