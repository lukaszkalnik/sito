/** Upper bound on remembered URLs; about the size of two large galleries. */
const MAX_REMEMBERED = 50_000;

/** The shape of a loaded thumbnail; a square one counts as landscape. */
export type ThumbnailShape = 'landscape' | 'portrait';

const loaded = new Map<string, ThumbnailShape>();

/** The shape of an image `width` × `height` pixels in size. */
export function shapeFrom(width: number, height: number): ThumbnailShape {
  return height > width ? 'portrait' : 'landscape';
}

/**
 * Remembers that the thumbnail at `src` has been shown once, and its shape.
 * Virtualized tiles unmount when they scroll away; when one comes back, the
 * webview still has the image cached, so it can be shown at once in its shape
 * without the loading delay and fade-in.
 */
export function rememberLoaded(src: string, shape: ThumbnailShape): void {
  if (loaded.size >= MAX_REMEMBERED) loaded.clear();
  loaded.set(src, shape);
}

/** The shape of the thumbnail at `src` if it has been shown before. */
export function shapeOf(src: string): ThumbnailShape | undefined {
  return loaded.get(src);
}

/** Forgets all remembered thumbnails. For tests. */
export function forgetLoaded(): void {
  loaded.clear();
}
