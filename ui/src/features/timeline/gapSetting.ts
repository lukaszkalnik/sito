/** Smallest time gap the slider offers, in seconds. */
export const MIN_GAP = 1;
/** Largest time gap the slider offers, in seconds. */
export const MAX_GAP = 60;
/** Gap used for a folder without a stored value; matches `DEFAULT_MAX_GAP` in core. */
export const DEFAULT_GAP = 10;

const STORAGE_KEY = 'sito.timeGap';

/** Rounds and clamps `seconds` to the slider range; anything else becomes the default. */
export function clampGap(seconds: unknown): number {
  if (typeof seconds !== 'number' || !Number.isFinite(seconds)) return DEFAULT_GAP;
  return Math.min(MAX_GAP, Math.max(MIN_GAP, Math.round(seconds)));
}

function readAll(): Record<string, unknown> {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}');
    return typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : {};
  } catch {
    return {};
  }
}

/** The time gap last chosen for `folder`, or the default. */
export function loadGap(folder: string): number {
  return clampGap(readAll()[folder]);
}

/** Remembers the time gap for `folder`. Storage failures are ignored. */
export function saveGap(folder: string, seconds: number): void {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ ...readAll(), [folder]: clampGap(seconds) }),
    );
  } catch {
    // Storage may be unavailable or full; the gap then only lasts until restart.
  }
}
