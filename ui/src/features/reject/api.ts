import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { RejectDestination } from './destination';

/** A rejected photo. Mirrors `RejectedDto`. */
export interface Rejected {
  name: string;
}

/**
 * What undoing did: a photo moved back from the "Odrzucone" folder or the
 * trash, or an accepted mark taken off or put back. Mirrors `UndoneKind`.
 */
export type UndoneKind = 'restored' | 'unaccepted' | 'reaccepted';

/** The action "Cofnij" undid. Mirrors `UndoneDto`. */
export interface Undone {
  /** Timeline id of the photo, or `null` if it could not be read back or is not on the timeline. */
  id: number | null;
  name: string;
  kind: UndoneKind;
}

const REJECT_ERROR_CODES = [
  'occupied',
  'missing',
  'missingFromTrash',
  'noTrash',
  'trashUndoUnsupported',
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
 * Undoes the newest action in the open folder: moves a rejected photo back,
 * or reverts an accept or un-accept; `null` if there is none.
 */
export async function undoLast(): Promise<Undone | null> {
  return invoke<Undone | null>('undo_last');
}

/** Whether there is an action to undo in the open folder. */
export async function canUndo(): Promise<boolean> {
  const state = await invoke<{ canUndo: boolean }>('get_undo_state');
  return state.canUndo;
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
