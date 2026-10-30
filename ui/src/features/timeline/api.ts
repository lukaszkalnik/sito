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
  /** What kind of files the photo has. */
  format: PhotoFormat;
}

/** What kind of files a photo has. Mirrors `PhotoFormatDto`. */
export type PhotoFormat = 'jpeg' | 'raw' | 'rawJpeg' | 'tiff';

/** Where a photo's capture time comes from. Mirrors `TimeSourceDto`. */
export type TimeSource = 'exif' | 'takeout' | 'takeoutCorrected' | 'file';

/** Returns the timeline of the last finished scan, newest first. */
export async function getTimeline(): Promise<TimelineEntry[]> {
  return invoke<TimelineEntry[]>('get_timeline');
}

/**
 * What a series' header says about its camera: its name (or that it is
 * unknown) when the series overlaps a series of another camera. Mirrors
 * `SeriesLabelDto`.
 */
export type SeriesLabel =
  | { readonly kind: 'plain' }
  | { readonly kind: 'camera'; readonly name: string }
  | { readonly kind: 'unknownCamera' };

/**
 * A series: `len` consecutive entries starting at `start`, counted in display
 * order. Mirrors `SeriesDto`.
 */
export interface Series {
  readonly start: number;
  readonly len: number;
  readonly label: SeriesLabel;
}

/** The series and the display order of the timeline entries. Mirrors `GroupingDto`. */
export interface Grouping {
  /** Indices of the timeline entries in the order they are shown. */
  readonly order: readonly number[];
  /** The series as ranges of display positions, sorted by start. */
  readonly series: readonly Series[];
}

/**
 * Returns the series of the last finished scan, each camera's photos at most
 * `maxGapSeconds` apart (the backend default when omitted), with the order
 * to show the timeline entries in.
 */
export async function getSeries(maxGapSeconds?: number): Promise<Grouping> {
  return invoke<Grouping>('get_series', { maxGapSeconds });
}
