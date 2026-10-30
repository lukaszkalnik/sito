/** Where rejected photos go. Mirrors `Destination` in `src-tauri/src/reject.rs`. */
export type RejectDestination = 'folder' | 'trash';

const STORAGE_KEY = 'sito.rejectDestination';

/** `value` as a destination; anything unknown is the safe default, the folder. */
export function parseDestination(value: unknown): RejectDestination {
  return value === 'trash' ? 'trash' : 'folder';
}

/** The destination last chosen, or the folder. */
export function loadDestination(): RejectDestination {
  try {
    return parseDestination(localStorage.getItem(STORAGE_KEY));
  } catch {
    return 'folder';
  }
}

/** Remembers the destination. Storage failures are ignored. */
export function saveDestination(destination: RejectDestination): void {
  try {
    localStorage.setItem(STORAGE_KEY, destination);
  } catch {
    // Storage may be unavailable; the choice then only lasts until restart.
  }
}

/**
 * Calls `handler` when another window (the viewer or the main window) saves
 * a destination; returns the function that stops listening.
 */
export function onDestinationChanged(handler: (destination: RejectDestination) => void) {
  const listener = (event: StorageEvent) => {
    if (event.key === STORAGE_KEY) handler(parseDestination(event.newValue));
  };
  window.addEventListener('storage', listener);
  return () => window.removeEventListener('storage', listener);
}
