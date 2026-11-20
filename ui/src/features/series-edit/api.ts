import { invoke } from '@tauri-apps/api/core';

/** An edit of the series, naming photos by their timeline ids. Mirrors `EditDto`. */
export type SeriesEdit =
  /** Splits the photo's series before it. */
  | { readonly kind: 'cut'; readonly photo: number }
  /** Takes the photo out of its series. */
  | { readonly kind: 'remove'; readonly photo: number }
  /** Puts the photo into the series of `into`. */
  | { readonly kind: 'join'; readonly photo: number; readonly into: number }
  /** Makes every photo of the series of `photo` a single photo. */
  | { readonly kind: 'ungroup'; readonly photo: number }
  /** Gives the photo's edited series, or the photo pinned on its own, back to automatic grouping. */
  | { readonly kind: 'restore'; readonly photo: number };

/** The outcome of an edit. Mirrors `EditedDto`. */
export interface Edited {
  /** File name of the photo the edit was about. */
  readonly name: string;
  /** Ids of the photos that look different now, newest first. */
  readonly changed: readonly number[];
}

const EDIT_ERROR_CODES = ['seriesChanged', 'internal'] as const;

/** Error codes of `edit_series`, see `CommandError`. */
export type EditErrorCode = (typeof EDIT_ERROR_CODES)[number];

/**
 * Applies `edit` to the series as shown for a gap of `maxGapSeconds`. All
 * windows are then told that the timeline changed.
 */
export async function editSeries(edit: SeriesEdit, maxGapSeconds: number): Promise<Edited> {
  const edited = await invoke<unknown>('edit_series', { edit, maxGapSeconds });
  const { name, changed } = (edited ?? {}) as { name?: unknown; changed?: unknown };
  if (typeof name !== 'string') throw new Error('edit_series returned no name');
  return { name, changed: idsOf(changed) };
}

/** `value` as a list of photo ids; anything else is no ids. */
function idsOf(value: unknown): number[] {
  return Array.isArray(value) ? value.filter((id): id is number => typeof id === 'number') : [];
}

/** The error code of a failed edit; unknown codes are `internal`. */
export function editErrorCode(error: unknown): EditErrorCode {
  const code = (error as { code?: unknown } | null)?.code;
  return EDIT_ERROR_CODES.find((known) => known === code) ?? 'internal';
}
