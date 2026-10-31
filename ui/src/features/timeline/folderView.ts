import type { GridPosition } from './gridPosition';
import type { TimelineMode } from './layout';

/** How many folders keep their view; the ones used longest ago are forgotten. */
export const MAX_FOLDERS = 50;

const STORAGE_KEY = 'sito.folderView';

/** How a folder was last shown: its view and where the grid was scrolled to (`null`: the top). */
export interface FolderView {
  readonly mode: TimelineMode;
  readonly position: GridPosition | null;
}

const DEFAULT_VIEW: FolderView = { mode: 'series', position: null };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function parseMode(value: unknown): TimelineMode {
  return value === 'all' ? 'all' : 'series';
}

function parsePosition(value: unknown): GridPosition | null {
  if (!isRecord(value)) return null;
  const { path, takenAt, offset } = value;
  if (typeof path !== 'string' || typeof takenAt !== 'string') return null;
  if (typeof offset !== 'number' || !Number.isFinite(offset)) return null;
  return { path, takenAt, offset };
}

function parseView(value: unknown): FolderView {
  if (!isRecord(value)) return DEFAULT_VIEW;
  return { mode: parseMode(value.mode), position: parsePosition(value.position) };
}

/** All stored views, by folder, in the order the folders were last used. */
function readAll(): Record<string, unknown> {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}');
    return isRecord(parsed) ? parsed : {};
  } catch {
    return {};
  }
}

/** The view `folder` was last shown in, or the default (series, at the top). */
export function loadFolderView(folder: string): FolderView {
  return parseView(readAll()[folder]);
}

/**
 * Stores `change` for `folder`, marks the folder as used last and forgets
 * all but the {@link MAX_FOLDERS} used last.
 */
function saveFolderView(folder: string, change: Partial<FolderView>): void {
  const all = readAll();
  const view = { ...parseView(all[folder]), ...change };
  // Folder paths are never integer-like keys, so the object keeps insertion order.
  const kept = Object.entries(all)
    .filter(([key]) => key !== folder)
    .slice(-(MAX_FOLDERS - 1));
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ ...Object.fromEntries(kept), [folder]: view }),
    );
  } catch {
    // Storage may be unavailable or full; the view then only lasts until restart.
  }
}

/** Remembers the view (mode) `folder` is shown in. Storage failures are ignored. */
export function saveFolderMode(folder: string, mode: TimelineMode): void {
  saveFolderView(folder, { mode });
}

/** Remembers where the grid of `folder` is scrolled to. Storage failures are ignored. */
export function saveFolderPosition(folder: string, position: GridPosition | null): void {
  saveFolderView(folder, { position });
}
