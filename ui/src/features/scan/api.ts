import { Channel, invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
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
  /** Shown from the cache; a background rescan is running and ends with `onScanRefreshed`. */
  refreshing: boolean;
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

/** Calls `handler` with the new summary when a background rescan has finished. */
export async function onScanRefreshed(
  handler: (summary: ScanSummary) => void,
): Promise<UnlistenFn> {
  return listen<ScanSummary>('scan-refreshed', (event) => handler(event.payload));
}
