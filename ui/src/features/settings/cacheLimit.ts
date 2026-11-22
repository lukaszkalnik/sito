/** The least space thumbnails may take, in MB. Mirrors `MIN_LIMIT_MB` in `src-tauri/src/cache_settings.rs`. */
export const MIN_LIMIT_MB = 50;

/**
 * The limit to apply once typing `text` ends: a whole number of MB, at least
 * {@link MIN_LIMIT_MB} (an empty or smaller value becomes that), or `current`
 * when `text` is not a number.
 */
export function finishedLimit(text: string, current: number): number {
  if (text.trim() === '') return MIN_LIMIT_MB;
  const mb = Number(text);
  if (!Number.isFinite(mb)) return current;
  return Math.max(MIN_LIMIT_MB, Math.round(mb));
}
