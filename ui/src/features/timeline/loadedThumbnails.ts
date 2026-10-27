/** Upper bound on remembered URLs; about the size of two large galleries. */
const MAX_REMEMBERED = 50_000;

const loaded = new Set<string>();

/**
 * Remembers that the thumbnail at `src` has been shown once. Virtualized
 * tiles unmount when they scroll away; when one comes back, the webview
 * still has the image cached, so it can be shown at once without the
 * loading delay and fade-in.
 */
export function rememberLoaded(src: string): void {
  if (loaded.size >= MAX_REMEMBERED) loaded.clear();
  loaded.add(src);
}

/** Whether the thumbnail at `src` has been shown before. */
export function wasLoaded(src: string): boolean {
  return loaded.has(src);
}

/** Forgets all remembered thumbnails. For tests. */
export function forgetLoaded(): void {
  loaded.clear();
}
