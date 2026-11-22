import { Channel, invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';

/** Where the cache is. Mirrors `LocationDto` in `src-tauri/src/cache_settings.rs`. */
export type CacheLocation =
  | { readonly kind: 'default' }
  | { readonly kind: 'chosen' }
  /** The chosen folder is missing; the default one is used this session. */
  | { readonly kind: 'fallback'; readonly chosen: string };

/** The system Sito runs on: the cache moves on Windows only. Mirrors `PlatformDto`. */
export type CachePlatform = 'windows' | 'macos' | 'other';

/** The cache settings. Mirrors `CacheSettingsDto`. */
export interface CacheSettings {
  /** The cache folder in use. */
  readonly folder: string;
  readonly location: CacheLocation;
  readonly platform: CachePlatform;
  /** The space thumbnails may take, in MB. */
  readonly thumbnailLimitMb: number;
}

/** The space the cache takes, in bytes. Mirrors `CacheSizeDto`. */
export interface CacheSize {
  readonly thumbnails: number;
  readonly photoData: number;
}

/** A folder that can take the cache. Mirrors `CheckedFolderDto`. */
export interface CheckedFolder {
  /** The cache folder that would be used (the chosen one's `Sito-cache`). */
  readonly target: string;
  /** On a network drive, which can slow Sito down. */
  readonly network: boolean;
  /** The size of the cache to move, in bytes. */
  readonly size: number;
}

/** How far a move has got. Mirrors `MoveProgressDto`. */
export interface MoveProgress {
  readonly done: number;
  readonly total: number;
}

/** Error codes of the cache commands. Mirrors `CacheError` in `src-tauri/src/cache_settings/error.rs`. */
export type CacheErrorCode =
  | 'notWritable'
  | 'insideOpenFolder'
  | 'insideCache'
  | 'link'
  | 'unsupportedName'
  | 'busy'
  | 'copyFailed'
  | 'copyDiffers'
  | 'cancelled'
  | 'revealFailed'
  | 'internal';

const errorCodes: readonly CacheErrorCode[] = [
  'notWritable',
  'insideOpenFolder',
  'insideCache',
  'link',
  'unsupportedName',
  'busy',
  'copyFailed',
  'copyDiffers',
  'cancelled',
  'revealFailed',
  'internal',
];

/** Event of the menu item „Ustawienia → Pamięć podręczna…”. Mirrors `SHOW_CACHE_SETTINGS_EVENT`. */
const SHOW_CACHE_SETTINGS_EVENT = 'settings-show-cache';

/** Error thrown by the cache commands; carries a code the UI translates. */
export class CacheError extends Error {
  readonly code: CacheErrorCode;

  constructor(code: CacheErrorCode) {
    super(`Cache command failed: ${code}`);
    this.name = 'CacheError';
    this.code = code;
  }
}

function toCacheError(raw: unknown): CacheError {
  if (typeof raw === 'object' && raw !== null && 'code' in raw) {
    const code = (raw as { code: unknown }).code;
    if (errorCodes.includes(code as CacheErrorCode)) return new CacheError(code as CacheErrorCode);
  }
  console.error('Unexpected cache command error', raw);
  return new CacheError('internal');
}

/** Runs `command`, turning its error into a {@link CacheError}. */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (raw) {
    throw toCacheError(raw);
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function parseLocation(raw: unknown): CacheLocation {
  if (isRecord(raw)) {
    if (raw.kind === 'fallback' && typeof raw.chosen === 'string') {
      return { kind: 'fallback', chosen: raw.chosen };
    }
    if (raw.kind === 'chosen') return { kind: 'chosen' };
  }
  return { kind: 'default' };
}

function parsePlatform(raw: unknown): CachePlatform {
  return raw === 'windows' || raw === 'macos' ? raw : 'other';
}

/** `raw` as {@link CacheSettings}, checked field by field. */
export function parseSettings(raw: unknown): CacheSettings {
  if (!isRecord(raw) || typeof raw.folder !== 'string') {
    throw new CacheError('internal');
  }
  return {
    folder: raw.folder,
    location: parseLocation(raw.location),
    platform: parsePlatform(raw.platform),
    thumbnailLimitMb: typeof raw.thumbnailLimitMb === 'number' ? raw.thumbnailLimitMb : 500,
  };
}

/** The cache settings. */
export async function getCacheSettings(): Promise<CacheSettings> {
  return parseSettings(await call<unknown>('get_cache_settings'));
}

/** How much space the cache takes now. */
export async function measureCache(): Promise<CacheSize> {
  return call<CacheSize>('measure_cache');
}

/** Lets thumbnails take `mb` MB; resolves to the limit in effect. */
export async function setThumbnailLimit(mb: number): Promise<number> {
  return call<number>('set_thumbnail_limit', { mb });
}

/** Removes every cached thumbnail. */
export async function clearThumbnails(): Promise<void> {
  await call('clear_thumbnails');
}

/** Empties the photo cache. */
export async function clearPhotoData(): Promise<void> {
  await call('clear_photo_data');
}

/** Shows the cache folder in the system's file manager. */
export async function revealCacheFolder(): Promise<void> {
  await call('reveal_cache_folder');
}

/** Whether `folder` can take the cache, and what the confirmation shows. */
export async function checkCacheFolder(folder: string): Promise<CheckedFolder> {
  return call<CheckedFolder>('check_cache_folder', { folder });
}

/**
 * Moves the cache into `folder` (its `Sito-cache`), or back to the default
 * folder for `null`, calling `onProgress` while copying.
 */
export async function moveCache(
  folder: string | null,
  onProgress: (progress: MoveProgress) => void,
): Promise<CacheSettings> {
  const channel = new Channel<MoveProgress>();
  channel.onmessage = onProgress;
  return parseSettings(await call<unknown>('move_cache', { folder, onProgress: channel }));
}

/** Stops a move that is copying. */
export async function cancelCacheMove(): Promise<void> {
  await call('cancel_cache_move');
}

/** Opens the system folder picker. Resolves to `null` if the user cancels. */
export async function pickCacheFolder(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === 'string' ? selected : null;
}

/** Calls `handler` whenever the user picks „Pamięć podręczna…” in the menu. */
export async function onShowCacheSettings(handler: () => void): Promise<UnlistenFn> {
  return listen(SHOW_CACHE_SETTINGS_EVENT, () => handler());
}
