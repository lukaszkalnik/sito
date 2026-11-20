import type { Arranged, EditAction, EditOption } from '../series-edit/editOptions';
import { photoEditOptions } from '../series-edit/editOptions';
import type { ViewerPhoto } from './api';

/**
 * The edits the viewer offers: only those that stay within the strip, plus
 * „Przywróć automatyczne grupowanie”, the other way back from an edit.
 * Joins move a photo into a series the viewer does not show.
 */
const VIEWER_ACTIONS: ReadonlySet<EditAction> = new Set(['cut', 'remove', 'ungroup', 'restore']);

/** The page's photos as the grid would arrange them: the strip, or the photo alone. */
function arrangedOf(photo: ViewerPhoto): Arranged {
  if (photo.series.length === 0) {
    return {
      entries: [photo.entry],
      series: [],
      pinnedIds: new Set(photo.pinned ? [photo.entry.id] : []),
    };
  }
  return {
    entries: photo.series,
    series: [
      { start: 0, len: photo.series.length, label: { kind: 'plain' }, edited: photo.seriesEdited },
    ],
    pinnedIds: new Set(),
  };
}

/**
 * The edits the viewer offers for photo `id`, the shown photo of `photo` or
 * another photo of its strip, by the grid's rules (see
 * {@link photoEditOptions}), without joins and without „Rozdziel serię” on
 * a series of one (whether that is a series at all is decided in slice 63).
 */
export function viewerEditOptions(photo: ViewerPhoto, id: number): EditOption[] {
  const ofSeriesOfOne = photo.series.length === 1;
  return photoEditOptions(arrangedOf(photo), id).filter(
    (option) =>
      VIEWER_ACTIONS.has(option.action) && !(ofSeriesOfOne && option.action === 'ungroup'),
  );
}

/**
 * The photo to show once the shown photo `id` is taken out of the series
 * `strip` (newest first): the photo after it, else the one before it;
 * `null` if `id` is not in the strip or no other photo is left.
 */
export function afterRemove(strip: readonly { readonly id: number }[], id: number): number | null {
  const position = strip.findIndex((member) => member.id === id);
  if (position < 0) return null;
  return (strip[position + 1] ?? strip[position - 1])?.id ?? null;
}
