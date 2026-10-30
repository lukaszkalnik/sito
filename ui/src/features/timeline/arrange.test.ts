import { describe, expect, it } from 'vitest';

import type { TimelineEntry } from './api';
import { arrangeEntries } from './arrange';

function entries(count: number): TimelineEntry[] {
  return Array.from({ length: count }, (_, id) => ({
    id,
    name: `${id}.jpg`,
    relativeDir: '',
    relativePath: `${id}.jpg`,
    takenAt: '2024-06-01T12:00:00',
    timeSource: 'exif',
    format: 'jpeg',
  }));
}

const ids = (list: readonly TimelineEntry[] | null) => list?.map((entry) => entry.id) ?? null;

describe('arrangeEntries', () => {
  it('puts the entries in display order', () => {
    expect(ids(arrangeEntries(entries(4), [0, 2, 1, 3]))).toEqual([0, 2, 1, 3]);
  });

  it('rejects an order that misses, repeats or invents an entry', () => {
    expect(arrangeEntries(entries(3), [0, 1])).toBeNull();
    expect(arrangeEntries(entries(3), [0, 1, 1])).toBeNull();
    expect(arrangeEntries(entries(3), [0, 1, 3])).toBeNull();
  });

  it('arranges an empty timeline', () => {
    expect(arrangeEntries([], [])).toEqual([]);
  });
});
