import { Channel, invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** How an update is installed. Mirrors `InstallKind` in `src-tauri/src/update.rs`. */
export type InstallKind = 'inApp' | 'manual';

/** A newer version found by the check. Mirrors `UpdateInfo`. */
export interface UpdateInfo {
  readonly version: string;
  /** The release notes (English, plain text), if the release has any. */
  readonly notes: string | null;
  readonly install: InstallKind;
}

/** The answer of a check. Mirrors `UpdateCheck`. */
export interface UpdateCheck {
  readonly currentVersion: string;
  /** The newer version, or `null` if the running one is the newest. */
  readonly update: UpdateInfo | null;
}

/** Progress of installing an update. Mirrors `InstallProgress`. */
export type InstallProgress =
  | { readonly kind: 'downloading'; readonly downloaded: number; readonly total: number | null }
  | { readonly kind: 'installing' };

/** Error codes of the update commands. Mirrors `UpdateError`. */
export type UpdateErrorCode = 'network' | 'badSignature' | 'noUpdate' | 'unsupported' | 'internal';

const errorCodes: readonly UpdateErrorCode[] = [
  'network',
  'badSignature',
  'noUpdate',
  'unsupported',
  'internal',
];

/** Event of the menu item „Pomoc → Sprawdź aktualizacje…”. Mirrors `CHECK_REQUESTED_EVENT`. */
const CHECK_REQUESTED_EVENT = 'update-check-requested';

/** Error thrown by the update commands; carries a code the UI translates. */
export class UpdateError extends Error {
  readonly code: UpdateErrorCode;

  constructor(code: UpdateErrorCode) {
    super(`Update failed: ${code}`);
    this.name = 'UpdateError';
    this.code = code;
  }
}

function toUpdateError(raw: unknown): UpdateError {
  if (typeof raw === 'object' && raw !== null && 'code' in raw) {
    const code = (raw as { code: unknown }).code;
    if (errorCodes.includes(code as UpdateErrorCode)) {
      return new UpdateError(code as UpdateErrorCode);
    }
  }
  return new UpdateError('internal');
}

function isUpdateInfo(raw: unknown): raw is UpdateInfo {
  if (typeof raw !== 'object' || raw === null) return false;
  const info = raw as Partial<Record<keyof UpdateInfo, unknown>>;
  return (
    typeof info.version === 'string' &&
    (info.notes === null || typeof info.notes === 'string') &&
    (info.install === 'inApp' || info.install === 'manual')
  );
}

/** `raw` as an {@link UpdateCheck}; throws an `internal` error if it is not one. */
function toUpdateCheck(raw: unknown): UpdateCheck {
  const check = (raw ?? {}) as Partial<Record<keyof UpdateCheck, unknown>>;
  const valid =
    typeof check.currentVersion === 'string' &&
    (check.update === null || isUpdateInfo(check.update));
  if (!valid) throw new UpdateError('internal');
  return check as UpdateCheck;
}

/** Asks GitHub for a newer version than the running one. */
export async function checkForUpdate(): Promise<UpdateCheck> {
  let raw: unknown;
  try {
    raw = await invoke<unknown>('check_for_update');
  } catch (error) {
    throw toUpdateError(error);
  }
  return toUpdateCheck(raw);
}

/**
 * Downloads and installs the update found by the last check, calling
 * `onProgress` on the way. On Windows the app then closes and the installer
 * restarts it, so the promise only settles if installing fails.
 */
export async function installUpdate(
  onProgress: (progress: InstallProgress) => void,
): Promise<void> {
  const channel = new Channel<InstallProgress>();
  channel.onmessage = onProgress;
  try {
    await invoke('install_update', { onProgress: channel });
  } catch (error) {
    throw toUpdateError(error);
  }
}

/** Calls `handler` whenever the user picks „Sprawdź aktualizacje…” in the menu. */
export async function onCheckRequested(handler: () => void): Promise<UnlistenFn> {
  return listen(CHECK_REQUESTED_EVENT, () => handler());
}
