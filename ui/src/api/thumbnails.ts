import { convertFileSrc } from '@tauri-apps/api/core';

/**
 * URL of the thumbnail of timeline photo `id` from scan `scanId`, served by
 * the backend's `thumb://` protocol. The scan id keeps URLs of different scans
 * apart, so cached images never show a photo from an older scan.
 */
export function thumbnailUrl(scanId: number, id: number): string {
  return `${convertFileSrc(String(id), 'thumb')}?scan=${scanId}`;
}
