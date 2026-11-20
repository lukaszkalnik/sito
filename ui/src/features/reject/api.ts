import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { RejectDestination } from './destination';

/** A rejected photo. Mirrors `RejectedDto`. */
export interface Rejected {
  name: string;
}

/**
 * The rejected photo "Cofnij" moved back from the "Odrzucone" folder or the
 * trash. Mirrors `RestoredDto`.
 */
export interface Restored {
  readonly kind: 'rejection';
  /** Timeline id of the photo, or `null` if it could not be read back or is not on the timeline. */
  readonly id: number | null;
  readonly name: string;
  /** The name it came back under because its own was taken, `null` if it got its own back. */
  readonly restoredName: string | null;
}

const UNDONE_EDITS = ['cut', 'remove', 'join', 'ungroup', 'restore'] as const;

/** Which series edit was undone. Mirrors `EditKindDto`. */
export type UndoneEditKind = (typeof UNDONE_EDITS)[number];

/** A series edit "Cofnij" took back. Mirrors `EditUndoneDto`. */
export interface UndoneEdit {
  readonly kind: 'edit';
  readonly edit: UndoneEditKind;
  /** File name of the photo the edit was about. */
  readonly name: string;
  /** Ids of the photos that look different now, newest first. */
  readonly changed: readonly number[];
}

/** What "Cofnij" took back. Mirrors `UndoneDto`. */
export type Undone = Restored | UndoneEdit;

const REJECT_ERROR_CODES = [
  'occupied',
  'photoMissing',
  'missing',
  'missingFromTrash',
  'noTrash',
  'rejectedDirIsLink',
  'trashUndoUnsupported',
  'editUndoFailed',
  'io',
  'internal',
] as const;

/** Error codes of the reject commands, see `CommandError`. */
export type RejectErrorCode = (typeof REJECT_ERROR_CODES)[number];

const TIMELINE_CHANGED = 'timeline-changed';

/** Moves photo `id` into the "Odrzucone" subfolder of its folder or into the system trash. */
export async function rejectPhoto(id: number, destination: RejectDestination): Promise<Rejected> {
  return invoke<Rejected>('reject_photo', { id, destination });
}

/**
 * Undoes the newest rejection or series edit in the open folder; `null` if
 * there is none. `maxGapSeconds` is the gap the window shows its series
 * with, which decides the photos an undone edit reports as changed.
 */
export async function undo(maxGapSeconds: number): Promise<Undone | null> {
  return parseUndone(await invoke<unknown>('undo', { maxGapSeconds }));
}

/** Reads the answer of `undo`; throws on one that is not an `UndoneDto`. */
export function parseUndone(value: unknown): Undone | null {
  if (value === null) return null;
  const undone = value as Record<string, unknown>;
  if (typeof undone.name !== 'string') throw new Error('undo returned no name');
  const name = undone.name;
  if (undone.kind === 'rejection') {
    const id = typeof undone.id === 'number' ? undone.id : null;
    const restoredName = typeof undone.restoredName === 'string' ? undone.restoredName : null;
    return { kind: 'rejection', id, name, restoredName };
  }
  const edit = UNDONE_EDITS.find((known) => known === undone.edit);
  const changed = undone.changed;
  if (undone.kind !== 'edit' || !edit || !Array.isArray(changed)) {
    throw new Error('undo returned an unknown outcome');
  }
  return { kind: 'edit', edit, name, changed: changed.filter((id) => typeof id === 'number') };
}

const UNDO_STATES = ['available', 'none', 'trashOnly'] as const;

/**
 * Whether "Cofnij" can undo something in the open folder: `available`,
 * `none` left, or `trashOnly` (only rejections into a trash this system
 * cannot restore from are left). Mirrors the `kind` of `UndoStateDto`.
 */
export type UndoState = (typeof UNDO_STATES)[number];

/** What "Cofnij" can undo in the open folder; an unknown answer counts as `none`. */
export async function undoState(): Promise<UndoState> {
  const state = await invoke<unknown>('get_undo_state');
  const kind = (state as { kind?: unknown } | null)?.kind;
  return UNDO_STATES.find((known) => known === kind) ?? 'none';
}

/** Calls `handler` whenever photos leave or return to the timeline (in any window). */
export async function onTimelineChanged(handler: () => void): Promise<UnlistenFn> {
  return listen(TIMELINE_CHANGED, () => handler());
}

/** The error code of a failed reject command; unknown codes are `internal`. */
export function rejectErrorCode(error: unknown): RejectErrorCode {
  const code = (error as { code?: unknown } | null)?.code;
  return REJECT_ERROR_CODES.find((known) => known === code) ?? 'internal';
}
