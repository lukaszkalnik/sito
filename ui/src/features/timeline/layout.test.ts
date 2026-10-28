import { describe, expect, it } from 'vitest';

import type { Series, TimelineEntry } from './api';
import {
  anchoredScrollTop,
  buildRows,
  columnsFor,
  DAY_ROW_HEIGHT,
  revealScrollTop,
  rowHeight,
  SERIES_END_SPACE,
  SERIES_PADDING,
  SERIES_ROW_HEIGHT,
  spansSeveralFolders,
  TILE_GAP,
  TILE_ROW_HEIGHT,
  TILE_WIDTH,
  tilesOf,
  totalHeight,
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
  /**
   * Rows as short labels: day date, 'series:<ids>', 'blocks:<ids>|<ids>', or
   * tile ids ('*' marks series tiles).
   */
  function shape(rows: TimelineRow[]) {
    return rows.map((r) => {
      if (r.kind === 'day') return r.date;
      if (r.kind === 'series') return `series:${r.entries.map((e) => e.id).join(',')}`;
      if (r.kind === 'blocks') {
        // Groups of single photos are marked with '·'.
        const groups = r.groups.map(
          (g) => `${g.series ? '' : '·'}${g.entries.map((e) => e.id).join(',')}`,
        );
        return `blocks:${groups.join('|')}`;
      }
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
    expect(shape(buildRows(entries, series, 3, 'series'))).toEqual(['2024-06-01', 'blocks:1,2,3']);
  });

  it('packs short series side by side while they fit the width', () => {
    // Pairs of photos, one pair per hour, all on one day.
    const pairs = Array.from({ length: 8 }, (_, i) =>
      entry(i, `2024-06-01T${String(20 - Math.floor(i / 2)).padStart(2, '0')}:00:0${i % 2}`),
    );
    const pairSeries = [0, 2, 4, 6].map((start) => ({ start, len: 2 }));
    // Five columns: two blocks of two need 2 × blockWidth(2) + gap = 696 px,
    // less than blockWidth(5) = 848 px; a third does not fit.
    expect(shape(buildRows(pairs, pairSeries, 5, 'series'))).toEqual([
      '2024-06-01',
      'blocks:0,1|2,3',
      'blocks:4,5|6,7',
    ]);
    // With the window's real width, more blocks share a row.
    expect(shape(buildRows(pairs, pairSeries, 5, 'series', 1100))).toEqual([
      '2024-06-01',
      'blocks:0,1|2,3|4,5',
      'blocks:6,7',
    ]);
  });

  it('lets single photos flow between blocks, but starts a new row for a new day', () => {
    const mixed = [
      entry(0, '2024-06-02T12:00:01'),
      entry(1, '2024-06-02T12:00:00'),
      entry(2, '2024-06-02T09:00:00'),
      entry(3, '2024-06-02T08:00:01'),
      entry(4, '2024-06-02T08:00:00'),
      entry(5, '2024-06-01T20:00:01'),
      entry(6, '2024-06-01T20:00:00'),
    ];
    const pairsOf = [0, 3, 5].map((start) => ({ start, len: 2 }));

    expect(shape(buildRows(mixed, pairsOf, 6, 'all'))).toEqual([
      '2024-06-02',
      'blocks:0,1|·2|3,4',
      '2024-06-01',
      'blocks:5,6',
    ]);
  });

  it('wraps single photos after a block onto the next row when they do not fit', () => {
    const photos = [
      entry(0, '2024-06-01T12:00:01'),
      entry(1, '2024-06-01T12:00:00'),
      ...[2, 3, 4, 5].map((id) => entry(id, `2024-06-01T1${9 - id}:00:00`)),
    ];
    // Three columns (512 px): a block of two (344 px) leaves room for no
    // single photo (8 px gap + 176 px); the singles fill the next row.
    expect(shape(buildRows(photos, [{ start: 0, len: 2 }], 3, 'all'))).toEqual([
      '2024-06-01',
      'blocks:0,1',
      [2, 3, 4],
      [5],
    ]);
    // Four columns (680 px): one single fits next to the block, not two.
    expect(shape(buildRows(photos, [{ start: 0, len: 2 }], 4, 'all'))).toEqual([
      '2024-06-01',
      'blocks:0,1|·2',
      [3, 4, 5],
    ]);
  });

  it('gives a series longer than a row its own header and tile rows', () => {
    expect(shape(buildRows(entries, series, 2, 'series'))).toEqual([
      '2024-06-01',
      'series:1,2,3',
      ['*1', '*2'],
      ['*3'],
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
      'blocks:0,1',
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

describe('anchoredScrollTop', () => {
  // Ten photos a second apart, one column, so every photo has its own row.
  const photos = Array.from({ length: 10 }, (_, id) =>
    entry(id, `2024-06-01T12:00:${String(59 - id).padStart(2, '0')}`),
  );
  const all = buildRows(photos, [], 1, 'all');
  const grouped: Series[] = [{ start: 6, len: 3 }];
  const allGrouped = buildRows(photos, grouped, 1, 'all');
  const onlySeries = buildRows(photos, grouped, 1, 'series');
  /** Offset of the tile row showing photo `id`. */
  const offsetOf = (rows: TimelineRow[], id: number) => {
    const index = rows.findIndex((r) => r.kind === 'tiles' && r.entries.some((e) => e.id === id));
    return totalHeight(rows.slice(0, index));
  };

  it('stays at the top', () => {
    expect(anchoredScrollTop(all, onlySeries, 0)).toBe(0);
  });

  it('keeps the top photo in place when series appear above it', () => {
    const scrollTop = offsetOf(all, 7) + 20;

    expect(anchoredScrollTop(all, allGrouped, scrollTop)).toBe(offsetOf(allGrouped, 7) + 20);
  });

  it('keeps the next shown photo in place when the top one is hidden', () => {
    // Photo 5 is a single, so photo 6 (the next row) is the first one left.
    const scrollTop = offsetOf(allGrouped, 5) + 50;
    const distance = offsetOf(allGrouped, 6) - scrollTop;
    expect(distance).toBeGreaterThan(0);

    expect(anchoredScrollTop(allGrouped, onlySeries, scrollTop)).toBe(
      Math.max(0, offsetOf(onlySeries, 6) - distance),
    );
  });

  it('keeps the next shown photo in place further down the list', () => {
    // Photo 9 is a single after the series; switching back to all photos keeps 7 in place.
    const scrollTop = offsetOf(onlySeries, 7) + 10;

    expect(anchoredScrollTop(onlySeries, allGrouped, scrollTop)).toBe(offsetOf(allGrouped, 7) + 10);
  });

  it('keeps a header above the top photo visible', () => {
    // Scrolled so that the series header of photo 6 is at the very top.
    const scrollTop = offsetOf(allGrouped, 6) - SERIES_ROW_HEIGHT;

    expect(anchoredScrollTop(allGrouped, onlySeries, scrollTop)).toBe(
      offsetOf(onlySeries, 6) - SERIES_ROW_HEIGHT,
    );
  });

  it('ignores a row whose tiles are scrolled out and only the gap below shows', () => {
    // Photo 6 is out of view; only the space below its tiles is at the top.
    const scrollTop = offsetOf(allGrouped, 7) - 4;

    expect(anchoredScrollTop(allGrouped, all, scrollTop)).toBe(offsetOf(all, 7) - 4);
  });

  it('goes to the end when nothing follows the top photo', () => {
    const scrollTop = offsetOf(all, 9);

    expect(anchoredScrollTop(all, onlySeries, scrollTop)).toBe(totalHeight(onlySeries));
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

  it('gives a row of blocks room for header, tiles and the space below', () => {
    const [, blocks] = buildRows(
      [entry(0, '2024-06-01T12:00:05'), entry(1, '2024-06-01T12:00:00')],
      [{ start: 0, len: 2 }],
      3,
      'all',
    ) as [TimelineRow, TimelineRow];

    expect(blocks.kind).toBe('blocks');
    expect(rowHeight(blocks)).toBe(SERIES_ROW_HEIGHT + TILE_ROW_HEIGHT + SERIES_END_SPACE);
  });
});

describe('anchoredScrollTop with blocks', () => {
  it('keeps a photo in place when its series turns from a band into a block', () => {
    const photos = [
      entry(0, '2024-06-01T12:00:09'),
      entry(1, '2024-06-01T12:00:08'),
      entry(2, '2024-06-01T09:00:00'),
      entry(3, '2024-06-01T08:00:00'),
    ];
    const pair: Series[] = [{ start: 2, len: 2 }];
    // One column: the pair is a header with two tile rows; three: one block.
    const narrow = buildRows(photos, pair, 1, 'all');
    const wide = buildRows(photos, pair, 3, 'all');
    const tileTop = (rows: TimelineRow[], id: number) => {
      let start = 0;
      for (const row of rows) {
        const { entries, offset } = tilesOf(row);
        if (entries.some((e) => e.id === id)) return start + offset;
        start += rowHeight(row);
      }
      throw new Error(`photo ${id} not shown`);
    };
    const scrollTop = tileTop(narrow, 2) - 10;

    expect(anchoredScrollTop(narrow, wide, scrollTop)).toBe(tileTop(wide, 2) - 10);
  });
});

describe('revealScrollTop', () => {
  const photos = Array.from({ length: 20 }, (_, id) =>
    entry(id, `2024-06-01T12:${String(59 - id).padStart(2, '0')}:00`),
  );
  // One column: a day header, then one row per photo.
  const rows = buildRows(photos, [], 1, 'all');
  const topOf = (id: number) => DAY_ROW_HEIGHT + id * TILE_ROW_HEIGHT;

  it('leaves the view alone when the photo is fully visible', () => {
    expect(revealScrollTop(rows, 3, topOf(2), 600)).toBe(topOf(2));
  });

  it('centres a photo that is above or below the view', () => {
    const tile = TILE_ROW_HEIGHT - TILE_GAP;
    expect(revealScrollTop(rows, 15, 0, 600)).toBe(topOf(15) - (600 - tile) / 2);
    expect(revealScrollTop(rows, 1, topOf(10), 600)).toBe(0);
  });

  it('is null for a photo the rows do not show', () => {
    expect(revealScrollTop(rows, 99, 0, 600)).toBeNull();
  });
});
