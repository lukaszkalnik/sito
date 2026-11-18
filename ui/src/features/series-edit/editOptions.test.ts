import { describe, expect, it } from 'vitest';

import type { Series, TimelineEntry } from '../timeline/api';
import { editOptionsFor, photoEditOptions, seriesEditOptions, type Arranged } from './editOptions';

function entry(id: number): TimelineEntry {
  return {
    id,
    name: `IMG_${id}.jpg`,
    relativeDir: '',
    relativePath: `IMG_${id}.jpg`,
    takenAt: '2024-06-01T12:00:00',
    timeSource: 'exif',
    format: 'jpeg',
  };
}

function series(start: number, len: number, edited = false): Series {
  return { start, len, label: { kind: 'plain' }, edited };
}

/**
 * Ids 10, 11, 12 … in display order: 10–12 one series, 13 single, 14–15 an
 * edited series, 16–17 a series right after it, 18 pinned on its own.
 */
const arranged: Arranged = {
  entries: [10, 11, 12, 13, 14, 15, 16, 17, 18].map(entry),
  series: [series(0, 3), series(4, 2, true), series(6, 2)],
  pinnedIds: new Set([18]),
};

const actions = (id: number) => photoEditOptions(arranged, id).map((o) => o.action);

describe('photoEditOptions', () => {
  it('offers no split before the first or the last photo of a series', () => {
    expect(actions(10)).toEqual(['remove', 'ungroup']);
    // Splitting before the last photo would only take it out.
    expect(actions(12)).toEqual(['remove', 'ungroup']);
  });

  it('splits a series before a photo in its middle', () => {
    expect(photoEditOptions(arranged, 11)[0]).toEqual({
      action: 'cut',
      edit: { kind: 'cut', photo: 11 },
    });
    expect(actions(11)).toEqual(['cut', 'remove', 'ungroup']);
  });

  it('only ungroups a series of two, where every edit leaves two single photos', () => {
    expect(actions(17)).toEqual(['ungroup']);
  });

  it('lets a single photo join the series before and after it', () => {
    expect(photoEditOptions(arranged, 13)).toEqual([
      { action: 'joinPrevious', edit: { kind: 'join', photo: 13, into: 12 } },
      { action: 'joinNext', edit: { kind: 'join', photo: 13, into: 14 } },
    ]);
  });

  it('lets the edge photo of a series join the neighbouring series', () => {
    expect(photoEditOptions(arranged, 15)).toContainEqual({
      action: 'joinNext',
      edit: { kind: 'join', photo: 15, into: 16 },
    });
    expect(actions(16)).toEqual(['joinPrevious', 'ungroup']);
  });

  it('offers restoring an edited series and a pinned single photo', () => {
    expect(actions(14)).toEqual(['ungroup', 'restore']);
    expect(actions(18)).toEqual(['joinPrevious', 'restore']);
  });

  it('offers nothing for a photo that is not shown', () => {
    expect(photoEditOptions(arranged, 99)).toEqual([]);
  });
});

describe('seriesEditOptions', () => {
  it('ungroups any series and restores an edited one', () => {
    expect(seriesEditOptions(arranged, 10).map((o) => o.action)).toEqual(['ungroup']);
    expect(seriesEditOptions(arranged, 14)).toEqual([
      { action: 'ungroup', edit: { kind: 'ungroup', photo: 14 } },
      { action: 'restore', edit: { kind: 'restore', photo: 14 } },
    ]);
  });

  it('offers nothing for a photo that does not start a series', () => {
    expect(seriesEditOptions(arranged, 11)).toEqual([]);
  });
});

describe('editOptionsFor', () => {
  it('picks the options by the menu target', () => {
    expect(editOptionsFor(arranged, { kind: 'photo', id: 13 })).toHaveLength(2);
    expect(editOptionsFor(arranged, { kind: 'series', firstId: 14 })).toHaveLength(2);
  });
});
