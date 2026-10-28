import { invoke } from '@tauri-apps/api/core';

/** One photo on the timeline. Mirrors `TimelineEntryDto`. */
export interface TimelineEntry {
  id: number;
  name: string;
  relativeDir: string;
  /** File path relative to the scanned folder, with the OS path separator. */
  relativePath: string;
  /** Local capture time, ISO 8601 without offset, e.g. `2024-06-01T12:00:05`. */
  takenAt: string;
  /** Where the capture time comes from. */
  timeSource: TimeSource;
}

/** Where a photo's capture time comes from. Mirrors `TimeSourceDto`. */
export type TimeSource = 'exif' | 'file';

/** Returns the timeline of the last finished scan, newest first. */
export async function getTimeline(): Promise<TimelineEntry[]> {
  return invoke<TimelineEntry[]>('get_timeline');
}

/** A series: `len` consecutive timeline entries starting at id `start`. Mirrors `SeriesDto`. */
export interface Series {
  start: number;
  len: number;
}

/**
 * Returns the series of the last finished scan: neighbouring photos at most
 * `maxGapSeconds` apart (the backend default when omitted).
 */
export async function getSeries(maxGapSeconds?: number): Promise<Series[]> {
  return invoke<Series[]>('get_series', { maxGapSeconds });
}
