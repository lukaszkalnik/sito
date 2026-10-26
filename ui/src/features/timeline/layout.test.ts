import { describe, expect, it } from 'vitest';

import type { TimelineEntry } from '../../api/scan';
import { buildRows, columnsFor, rowHeight, TILE_GAP, TILE_WIDTH } from './layout';

function entry(id: number, takenAt: string): TimelineEntry {
  return { id, name: `IMG_${id}.jpg`, relativeDir: '', takenAt, fromFileDate: false };
}

describe('columnsFor', () => {
  it('fits as many tiles as the width allows', () => {
    expect(columnsFor(TILE_WIDTH)).toBe(1);
    expect(columnsFor(2 * TILE_WIDTH + TILE_GAP)).toBe(2);
    expect(columnsFor(2 * TILE_WIDTH + TILE_GAP - 1)).toBe(1);
    // 6 × 160 px + 5 × 8 px gaps = 1000 px
    expect(columnsFor(1000)).toBe(6);
  });

  it('never returns less than one column', () => {
    expect(columnsFor(0)).toBe(1);
    expect(columnsFor(50)).toBe(1);
  });
});

describe('buildRows', () => {
  it('returns no rows for an empty timeline', () => {
    expect(buildRows([], 3)).toEqual([]);
  });

  it('starts every day with a header and wraps tiles into rows', () => {
    const entries = [
      entry(0, '2024-06-02T09:00:00'),
      entry(1, '2024-06-01T12:00:05'),
      entry(2, '2024-06-01T12:00:04'),
      entry(3, '2024-06-01T12:00:03'),
      entry(4, '2024-06-01T08:00:00'),
      entry(5, '2024-05-31T18:15:00'),
    ];

    const rows = buildRows(entries, 3);

    expect(rows.map((r) => (r.kind === 'day' ? r.date : r.entries.map((e) => e.id)))).toEqual([
      '2024-06-02',
      [0],
      '2024-06-01',
      [1, 2, 3],
      [4],
      '2024-05-31',
      [5],
    ]);
  });

  it('gives every row a unique key', () => {
    const entries = Array.from({ length: 50 }, (_, i) =>
      entry(i, `2024-06-${String(10 - Math.floor(i / 10)).padStart(2, '0')}T12:00:00`),
    );

    const keys = buildRows(entries, 4).map((r) => r.key);

    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe('rowHeight', () => {
  it('is smaller for day headers than for tile rows', () => {
    const rows = buildRows([entry(0, '2024-06-01T12:00:00')], 1);

    expect(rowHeight(rows[0]!)).toBeLessThan(rowHeight(rows[1]!));
  });
});
