import { describe, expect, it } from 'vitest';

import type { Series, TimelineEntry } from './api';
import { restoredScrollTop, topPosition, type GridPosition } from './gridPosition';
import {
  buildRows,
  DAY_ROW_HEIGHT,
  SERIES_ROW_HEIGHT,
  TILE_ROW_HEIGHT,
  type TimelineRow,
} from './layout';

/** Photo `n` of one day, a minute apart, newest first. */
function entry(n: number): TimelineEntry {
  return {
    id: 1000 + n,
    name: `IMG_${n}.jpg`,
    relativeDir: '',
    relativePath: `Camera\\IMG_${n}.jpg`,
    takenAt: `2024-06-01T12:${String(59 - n).padStart(2, '0')}:00`,
    timeSource: 'exif',
    format: 'jpeg',
  };
}

const entries = Array.from({ length: 30 }, (_, n) => entry(n));
/** One column: a day header, then one row per photo. */
const oneColumn = buildRows(entries, [], 1, 'all');
const topOf = (n: number) => DAY_ROW_HEIGHT + n * TILE_ROW_HEIGHT;

function at(n: number, offset: number): GridPosition {
  const { relativePath, takenAt } = entry(n);
  return { path: relativePath, takenAt, offset };
}

describe('topPosition', () => {
  it('is null at the very top', () => {
    expect(topPosition(oneColumn, 0)).toBeNull();
  });

  it('names the partly visible photo at the top and how far it is scrolled past', () => {
    expect(topPosition(oneColumn, topOf(4) + 50)).toEqual(at(4, 50));
  });

  it('skips the gap below a tile', () => {
    expect(topPosition(oneColumn, topOf(5) - 4)).toEqual(at(5, -4));
  });

  it('names the first photo below a day header at the top', () => {
    expect(topPosition(oneColumn, 10)).toEqual(at(0, 10 - DAY_ROW_HEIGHT));
  });

  it('is null when no photo is visible', () => {
    expect(topPosition([], 100)).toBeNull();
  });
});

describe('restoredScrollTop', () => {
  it('returns to the same scroll position in the same rows', () => {
    for (const scrollTop of [10, topOf(3), topOf(7) + 64, topOf(20) - 3]) {
      const position = topPosition(oneColumn, scrollTop);
      expect(position).not.toBeNull();
      expect(restoredScrollTop(oneColumn, position!)).toBe(scrollTop);
    }
  });

  it('finds the photo in rows of another width', () => {
    const threeColumns = buildRows(entries, [], 3, 'all');
    // Photo 7 is in the third row of tiles.
    expect(restoredScrollTop(threeColumns, at(7, 20))).toBe(
      DAY_ROW_HEIGHT + 2 * TILE_ROW_HEIGHT + 20,
    );
  });

  it('finds the photo in the series view', () => {
    const series: Series[] = [{ start: 10, len: 12, label: { kind: 'plain' } }];
    const rows = buildRows(entries, series, 4, 'series');
    // Day and series headers, then photos 10–13 above the row of photo 15.
    const rowTop = DAY_ROW_HEIGHT + SERIES_ROW_HEIGHT + TILE_ROW_HEIGHT;
    expect(restoredScrollTop(rows, at(15, 7))).toBe(rowTop + 7);
  });

  it('falls back to the next older photo when the photo is gone', () => {
    const rows = buildRows(
      entries.filter((e) => e.name !== 'IMG_6.jpg'),
      [],
      1,
      'all',
    );
    // Photo 7 moved up into photo 6's row.
    expect(restoredScrollTop(rows, at(6, 5))).toBe(topOf(6) + 5);
  });

  it('falls back to the last photo when all photos are newer', () => {
    const rows: TimelineRow[] = buildRows(entries.slice(0, 3), [], 1, 'all');
    const position = { path: 'Camera\\OLD.jpg', takenAt: '2020-01-01T00:00:00', offset: 0 };
    expect(restoredScrollTop(rows, position)).toBe(topOf(2));
  });

  it('is null when the rows show no photo', () => {
    expect(restoredScrollTop([], at(1, 0))).toBeNull();
  });

  it('never scrolls above the top', () => {
    expect(restoredScrollTop(oneColumn, at(0, -200))).toBe(0);
  });
});
