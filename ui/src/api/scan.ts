import { Channel, invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

/** Progress update streamed while a scan is running. Mirrors `ScanProgressDto`. */
export type ScanProgress =
  { phase: 'searching'; found: number } | { phase: 'reading'; done: number; total: number };

/** Result of a finished scan. Mirrors `ScanSummaryDto`. */
export interface ScanSummary {
  /** Identifies the scan; changes with every scan, also across app launches. */
  scanId: number;
  photosFound: number;
  unreadableEntries: number;
  withoutExifDate: number;
}

/** One photo on the timeline. Mirrors `TimelineEntryDto`. */
export interface TimelineEntry {
  id: number;
  name: string;
  relativeDir: string;
  /** File path relative to the scanned folder, with the OS path separator. */
  relativePath: string;
  /** Local capture time, ISO 8601 without offset, e.g. `2024-06-01T12:00:05`. */
  takenAt: string;
  /** `true` if the time comes from the file date instead of EXIF. */
  fromFileDate: boolean;
}

/** Error codes sent by the backend. Mirrors `CommandError`. */
export type ScanErrorCode = 'notFound' | 'notADirectory' | 'io' | 'internal';

const errorCodes: readonly ScanErrorCode[] = ['notFound', 'notADirectory', 'io', 'internal'];

/** Error thrown by {@link scanFolder}; carries a code the UI translates. */
export class ScanError extends Error {
  readonly code: ScanErrorCode;

  constructor(code: ScanErrorCode) {
    super(`Scan failed: ${code}`);
    this.name = 'ScanError';
    this.code = code;
  }
}

function toScanError(raw: unknown): ScanError {
  if (typeof raw === 'object' && raw !== null && 'code' in raw) {
    const code = (raw as { code: unknown }).code;
    if (errorCodes.includes(code as ScanErrorCode)) {
      return new ScanError(code as ScanErrorCode);
    }
  }
  return new ScanError('internal');
}

/** Opens the system folder picker. Resolves to `null` if the user cancels. */
export async function pickFolder(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === 'string' ? selected : null;
}

/** Builds the timeline of `path` in the backend, calling `onProgress` while scanning. */
export async function scanFolder(
  path: string,
  onProgress: (progress: ScanProgress) => void,
): Promise<ScanSummary> {
  const channel = new Channel<ScanProgress>();
  channel.onmessage = onProgress;
  try {
    return await invoke<ScanSummary>('start_scan', { path, onProgress: channel });
  } catch (raw) {
    throw toScanError(raw);
  }
}

/** Returns the timeline of the last finished scan, newest first. */
export async function getTimeline(): Promise<TimelineEntry[]> {
  return invoke<TimelineEntry[]>('get_timeline');
}
