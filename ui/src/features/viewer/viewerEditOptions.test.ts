import { describe, expect, it } from 'vitest';

import type { ViewerPhoto } from './api';
import { pageOf } from './photoViewerFixtures';
import { afterRemove, viewerEditOptions } from './viewerEditOptions';

/** The page of photo `shown` in a series of the photos `members` (none: a single photo). */
function page(shown: number, members: number[], changes: Partial<ViewerPhoto> = {}): ViewerPhoto {
  const base = pageOf(shown);
  return {
    ...base,
    series: members.map((id) => ({ ...base.entry, id, name: `IMG_${id}.jpg` })),
    ...changes,
  };
}

const actions = (photo: ViewerPhoto, id: number) =>
  viewerEditOptions(photo, id).map((option) => option.action);

describe('viewerEditOptions', () => {
  it('offers a cut in the middle, taking out and ungrouping in a long series', () => {
    const photo = page(2, [1, 2, 3, 4]);

    expect(actions(photo, 1)).toEqual(['remove', 'ungroup']);
    expect(actions(photo, 2)).toEqual(['cut', 'remove', 'ungroup']);
    expect(actions(photo, 3)).toEqual(['cut', 'remove', 'ungroup']);
    expect(actions(photo, 4)).toEqual(['remove', 'ungroup']);
  });

  it('offers a cut only in the middle of a series of three', () => {
    const photo = page(1, [1, 2, 3]);

    expect(actions(photo, 1)).toEqual(['remove', 'ungroup']);
    expect(actions(photo, 2)).toEqual(['cut', 'remove', 'ungroup']);
    expect(actions(photo, 3)).toEqual(['remove', 'ungroup']);
  });

  it('offers only ungrouping in a series of two', () => {
    const photo = page(1, [1, 2]);

    expect(actions(photo, 1)).toEqual(['ungroup']);
    expect(actions(photo, 2)).toEqual(['ungroup']);
  });

  it('offers the edits of another photo of the strip than the shown one', () => {
    const photo = page(1, [1, 2, 3]);

    const [cut] = viewerEditOptions(photo, 2);

    expect(cut?.edit).toEqual({ kind: 'cut', photo: 2 });
  });

  it('offers restoring on every photo of an edited series', () => {
    const photo = page(1, [1, 2, 3], { seriesEdited: true });

    expect(actions(photo, 1)).toEqual(['remove', 'ungroup', 'restore']);
    expect(actions(photo, 2)).toEqual(['cut', 'remove', 'ungroup', 'restore']);
    expect(actions(photo, 3)).toEqual(['remove', 'ungroup', 'restore']);
    // A whole-series edit names the series by its first photo, as in the grid.
    expect(viewerEditOptions(photo, 3).at(-1)?.edit).toEqual({ kind: 'restore', photo: 1 });
    expect(actions(page(1, [1, 2], { seriesEdited: true }), 2)).toEqual(['ungroup', 'restore']);
  });

  it('offers nothing on an unedited series of one, only restoring on an edited one', () => {
    expect(actions(page(1, [1]), 1)).toEqual([]);
    expect(actions(page(1, [1], { seriesEdited: true }), 1)).toEqual(['restore']);
  });

  it('offers nothing on a single photo, only restoring on a pinned one', () => {
    expect(actions(page(0, []), 0)).toEqual([]);
    expect(viewerEditOptions(page(0, [], { pinned: true }), 0)).toEqual([
      { action: 'restore', edit: { kind: 'restore', photo: 0 } },
    ]);
  });

  it('never offers a join, nor edits of a photo outside the strip', () => {
    const photo = page(2, [1, 2, 3], { seriesEdited: true, previous: 0, next: 4 });

    for (const id of [1, 2, 3]) {
      expect(actions(photo, id)).not.toContain('joinPrevious');
      expect(actions(photo, id)).not.toContain('joinNext');
    }
    expect(actions(photo, 4)).toEqual([]);
    expect(actions(page(0, [], { pinned: true }), 1)).toEqual([]);
  });
});

describe('afterRemove', () => {
  const strip = [{ id: 1 }, { id: 2 }, { id: 3 }];

  it('goes on to the next photo, or back to the previous one after the last', () => {
    expect(afterRemove(strip, 1)).toBe(2);
    expect(afterRemove(strip, 2)).toBe(3);
    expect(afterRemove(strip, 3)).toBe(2);
  });

  it('has nothing for a photo outside the strip or the only photo', () => {
    expect(afterRemove(strip, 9)).toBeNull();
    expect(afterRemove([{ id: 1 }], 1)).toBeNull();
  });
});
