import { describe, expect, it } from 'vitest';

import type { Series, TimelineEntry } from '../../api/scan';
import {
  buildRows,
  columnsFor,
  rowHeight,
  SERIES_PADDING,
  spansSeveralFolders,
  TILE_GAP,
  TILE_WIDTH,
  type TimelineRow,
} from './layout';

function entry(id: number, takenAt: string): TimelineEntry {
  return {
    id,
    name: `IMG_${id}.jpg`,
    relativeDir: '',
    relativePath: `IMG_${id}.jpg`,
    takenAt,
    fromFileDate: false,
  };
}

describe('spansSeveralFolders', () => {
  const inFolder = (id: number, relativeDir: string) => ({
    ...entry(id, '2024-06-01T12:00:00'),
    relativeDir,
  });

  it('is false for no photos and for photos from one folder', () => {
    expect(spansSeveralFolders([])).toBe(false);
    expect(spansSeveralFolders([inFolder(0, 'Camera'), inFolder(1, 'Camera')])).toBe(false);
    expect(spansSeveralFolders([inFolder(0, ''), inFolder(1, '')])).toBe(false);
  });

  it('is true as soon as one photo is in another folder', () => {
    expect(spansSeveralFolders([inFolder(0, 'Camera'), inFolder(1, 'Pulpit\\2024')])).toBe(true);
    expect(spansSeveralFolders([inFolder(0, ''), inFolder(1, 'Camera')])).toBe(true);
  });
});

describe('columnsFor', () => {
  const padding = 2 * SERIES_PADDING;

  it('fits as many tiles as the width allows, leaving room for series padding', () => {
    expect(columnsFor(TILE_WIDTH + padding)).toBe(1);
    expect(columnsFor(2 * TILE_WIDTH + TILE_GAP + padding)).toBe(2);
    expect(columnsFor(2 * TILE_WIDTH + TILE_GAP + padding - 1)).toBe(1);
    // 6 × 160 px + 5 × 8 px gaps + 2 × 8 px padding = 1016 px
    expect(columnsFor(1016)).toBe(6);
  });

  it('never returns less than one column', () => {
    expect(columnsFor(0)).toBe(1);
    expect(columnsFor(50)).toBe(1);
  });
});

describe('buildRows', () => {
  /** Rows as short labels: day date, 'series:<ids>', or tile ids ('*' marks series tiles). */
  function shape(rows: TimelineRow[]) {
    return rows.map((r) => {
      if (r.kind === 'day') return r.date;
      if (r.kind === 'series') return `series:${r.entries.map((e) => e.id).join(',')}`;
      return r.entries.map((e) => (r.inSeries ? `*${e.id}` : e.id));
    });
  }

  const entries = [
    entry(0, '2024-06-02T09:00:00'),
    entry(1, '2024-06-01T12:00:05'),
    entry(2, '2024-06-01T12:00:04'),
    entry(3, '2024-06-01T12:00:03'),
    entry(4, '2024-06-01T08:00:00'),
    entry(5, '2024-06-01T07:00:00'),
    entry(6, '2024-05-31T18:15:00'),
  ];
  const series: Series[] = [{ start: 1, len: 3 }];

  it('returns no rows for an empty timeline', () => {
    expect(buildRows([], [], 3, 'all')).toEqual([]);
    expect(buildRows([], [], 3, 'series')).toEqual([]);
  });

  it('starts every day with a header and wraps tiles into rows', () => {
    expect(shape(buildRows(entries, [], 3, 'all'))).toEqual([
      '2024-06-02',
      [0],
      '2024-06-01',
      [1, 2, 3],
      [4, 5],
      '2024-05-31',
      [6],
    ]);
  });

  it('puts every series on its own rows under a header in the full timeline', () => {
    expect(shape(buildRows(entries, series, 2, 'all'))).toEqual([
      '2024-06-02',
      [0],
      '2024-06-01',
      'series:1,2,3',
      ['*1', '*2'],
      ['*3'],
      [4, 5],
      '2024-05-31',
      [6],
    ]);
  });

  it('marks only the last row of a series as its end', () => {
    const seriesRows = buildRows(entries, series, 2, 'all').filter(
      (r) => r.kind === 'tiles' && r.inSeries,
    );

    expect(seriesRows.map((r) => r.kind === 'tiles' && r.seriesEnd)).toEqual([false, true]);
  });

  it('shows only series, and only days with series, in series mode', () => {
    expect(shape(buildRows(entries, series, 3, 'series'))).toEqual([
      '2024-06-01',
      'series:1,2,3',
      ['*1', '*2', '*3'],
    ]);
  });

  it('keeps a series crossing midnight together under its newest day', () => {
    const night = [
      entry(0, '2024-06-02T00:00:03'),
      entry(1, '2024-06-01T23:59:58'),
      entry(2, '2024-06-01T20:00:00'),
    ];

    expect(shape(buildRows(night, [{ start: 0, len: 2 }], 3, 'all'))).toEqual([
      '2024-06-02',
      'series:0,1',
      ['*0', '*1'],
      '2024-06-01',
      [2],
    ]);
  });

  it('gives every row a unique key', () => {
    const many = Array.from({ length: 50 }, (_, i) =>
      entry(i, `2024-06-${String(10 - Math.floor(i / 10)).padStart(2, '0')}T12:00:00`),
    );
    const manySeries = [
      { start: 3, len: 4 },
      { start: 20, len: 9 },
    ];

    for (const mode of ['all', 'series'] as const) {
      const keys = buildRows(many, manySeries, 4, mode).map((r) => r.key);
      expect(new Set(keys).size).toBe(keys.length);
    }
  });
});

describe('rowHeight', () => {
  it('gives headers less height than tile rows and adds space after a series', () => {
    const rows = buildRows(
      [entry(0, '2024-06-01T12:00:05'), entry(1, '2024-06-01T12:00:00')],
      [{ start: 0, len: 2 }],
      1,
      'all',
    );
    const [day, header, firstTiles, lastTiles] = rows as [
      TimelineRow,
      TimelineRow,
      TimelineRow,
      TimelineRow,
    ];

    expect(rowHeight(day)).toBeLessThan(rowHeight(firstTiles));
    expect(rowHeight(header)).toBeLessThan(rowHeight(firstTiles));
    expect(rowHeight(lastTiles)).toBeGreaterThan(rowHeight(firstTiles));
  });
});
