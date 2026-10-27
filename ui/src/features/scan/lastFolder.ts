const STORAGE_KEY = 'sito.lastFolder';

/** The folder opened last, to reopen on start; `null` if none. */
export function loadLastFolder(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

/** Remembers `folder` as the one to reopen on start. Storage failures are ignored. */
export function saveLastFolder(folder: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, folder);
  } catch {
    // Storage may be unavailable; Sito then starts with the folder picker.
  }
}
