import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** A photo moved into the "Odrzucone" folder. Mirrors `RejectedDto`. */
export interface Rejected {
  name: string;
}

/** A photo moved back from the "Odrzucone" folder. Mirrors `UndoneDto`. */
export interface Undone {
  /** Timeline id of the photo, or `null` if it could not be read back. */
  id: number | null;
  name: string;
}

/** Error codes of the reject commands, see `CommandError`. */
export type RejectErrorCode = 'occupied' | 'missing' | 'io' | 'internal';

const TIMELINE_CHANGED = 'timeline-changed';

/** Moves photo `id` into the "Odrzucone" subfolder of its folder. */
export async function rejectPhoto(id: number): Promise<Rejected> {
  return invoke<Rejected>('reject_photo', { id });
}

/** Moves the most recently rejected photo of the open folder back; `null` if none. */
export async function undoReject(): Promise<Undone | null> {
  return invoke<Undone | null>('undo_reject');
}

/** Whether there is a rejection to undo in the open folder. */
export async function canUndo(): Promise<boolean> {
  const state = await invoke<{ canUndo: boolean }>('get_undo_state');
  return state.canUndo;
}

/** Calls `handler` whenever photos leave or return to the timeline (in any window). */
export async function onTimelineChanged(handler: () => void): Promise<UnlistenFn> {
  return listen(TIMELINE_CHANGED, () => handler());
}

/** The error code of a failed reject command. */
export function rejectErrorCode(error: unknown): RejectErrorCode {
  const code = (error as { code?: unknown } | null)?.code;
  return code === 'occupied' || code === 'missing' || code === 'io' ? code : 'internal';
}
