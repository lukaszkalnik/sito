import type { Series, TimelineEntry } from '../timeline/api';
import type { MenuTarget } from '../timeline/menuRequest';
import type { SeriesEdit } from './api';

/** The edits the menus offer. */
export type EditAction = 'cut' | 'remove' | 'joinPrevious' | 'joinNext' | 'ungroup' | 'restore';

/** One entry of an edit menu: what it says and the edit it makes. */
export interface EditOption {
  readonly action: EditAction;
  readonly edit: SeriesEdit;
}

/** The timeline as shown: entries in display order, series as ranges of it. */
export interface Arranged {
  readonly entries: readonly TimelineEntry[];
  readonly series: readonly Series[];
  /** Ids of the photos pinned as single photos by an edit. */
  readonly pinnedIds: ReadonlySet<number>;
}

/** The series covering display position `position`, if any. */
function seriesAt(series: readonly Series[], position: number): Series | undefined {
  return series.find((s) => position >= s.start && position < s.start + s.len);
}

/** The edits of the series `own` as a whole. */
function wholeSeriesOptions(entries: readonly TimelineEntry[], own: Series): EditOption[] {
  const first = entries[own.start];
  if (!first) return [];
  const photo = first.id;
  const options: EditOption[] = [{ action: 'ungroup', edit: { kind: 'ungroup', photo } }];
  if (own.edited) options.push({ action: 'restore', edit: { kind: 'restore', photo } });
  return options;
}

/** Joining the series of the photo shown at `neighbour`, if it is another series than `own`. */
function joinOption(
  arranged: Arranged,
  photo: number,
  own: Series | undefined,
  neighbour: number,
  action: 'joinPrevious' | 'joinNext',
): EditOption[] {
  const next = arranged.entries[neighbour];
  const theirs = seriesAt(arranged.series, neighbour);
  if (!next || !theirs || theirs === own) return [];
  return [{ action, edit: { kind: 'join', photo, into: next.id } }];
}

/**
 * Splitting the series `own` before the photo `id` at `position`, and taking
 * the photo out, where they do something „Rozdziel serię” or the other does
 * not: a series of one or two photos only offers „Rozdziel serię” (any of the
 * three leaves single photos), and splitting before the last photo is taking
 * it out.
 */
function partOptions(own: Series | undefined, position: number, id: number): EditOption[] {
  if (!own || own.len <= 2) return [];
  const last = own.start + own.len - 1;
  const options: EditOption[] = [];
  if (position > own.start && position < last) {
    options.push({ action: 'cut', edit: { kind: 'cut', photo: id } });
  }
  options.push({ action: 'remove', edit: { kind: 'remove', photo: id } });
  return options;
}

/**
 * The edits offered for photo `id`: splitting its series before it and
 * taking it out (see {@link partOptions}), joining the series of the photo
 * shown before or after it when that is another series, and the edits of its
 * whole series. A single photo pinned by an edit can be restored.
 */
export function photoEditOptions(arranged: Arranged, id: number): EditOption[] {
  const position = arranged.entries.findIndex((entry) => entry.id === id);
  if (position < 0) return [];
  const own = seriesAt(arranged.series, position);
  const options: EditOption[] = [...partOptions(own, position, id)];
  options.push(...joinOption(arranged, id, own, position - 1, 'joinPrevious'));
  options.push(...joinOption(arranged, id, own, position + 1, 'joinNext'));
  if (own) options.push(...wholeSeriesOptions(arranged.entries, own));
  else if (arranged.pinnedIds.has(id)) {
    options.push({ action: 'restore', edit: { kind: 'restore', photo: id } });
  }
  return options;
}

/** The edits offered for the series whose first photo is `firstId`. */
export function seriesEditOptions(arranged: Arranged, firstId: number): EditOption[] {
  const own = arranged.series.find((s) => arranged.entries[s.start]?.id === firstId);
  return own ? wholeSeriesOptions(arranged.entries, own) : [];
}

/** The edits offered for a menu of the grid. */
export function editOptionsFor(arranged: Arranged, target: MenuTarget): EditOption[] {
  switch (target.kind) {
    case 'photo':
      return photoEditOptions(arranged, target.id);
    case 'series':
      return seriesEditOptions(arranged, target.firstId);
  }
}
