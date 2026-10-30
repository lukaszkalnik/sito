import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** A photo whose accepted mark changed. Mirrors `AcceptedDto`. */
export interface Accepted {
  name: string;
}

const ACCEPTED_CHANGED = 'accepted-changed';

/** Marks photo `id` of the current scan as accepted. */
export async function acceptPhoto(id: number): Promise<Accepted> {
  return invoke<Accepted>('accept_photo', { id });
}

/** Takes the accepted mark off photo `id` of the current scan. */
export async function unacceptPhoto(id: number): Promise<Accepted> {
  return invoke<Accepted>('unaccept_photo', { id });
}

/** Timeline ids of the accepted photos of the current scan. */
export async function getAcceptedIds(): Promise<readonly number[]> {
  const ids = await invoke<unknown>('get_accepted_ids');
  if (!Array.isArray(ids) || !ids.every((id) => typeof id === 'number')) {
    throw new Error('get_accepted_ids returned no list of ids');
  }
  return ids as readonly number[];
}

/** Calls `handler` whenever a photo is accepted or loses its mark (in any window). */
export async function onAcceptedChanged(handler: () => void): Promise<UnlistenFn> {
  return listen(ACCEPTED_CHANGED, () => handler());
}
