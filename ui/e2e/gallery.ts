import type { Page } from '@playwright/test';

import { mockTauri, type TauriMock } from './tauri-mock';

// 1×1 grey PNG, served for photos and thumbnails.
const PIXEL = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNoAAAAggCBd81ytgAAAABJRU5ErkJggg==',
  'base64',
);

// 30 photos (IMG_00030 is id 0) in moments of 3 shot 2 s apart, 10 minutes
// between moments: series at ids 0–2, 3–5 and so on, regrouped by the gap
// like the backend does when a photo is rejected.
export const BASE: TauriMock = {
  pickedFolder: 'C:\\Zdjęcia',
  scanProgress: [],
  scanResult: { ok: { scanId: 5, photosFound: 30, unreadableEntries: 0, withoutDate: 0 } },
  timeline: { count: 30, newest: '2024-06-01T12:00:00', stepSeconds: 0, stepPattern: [2, 2, 600] },
  series: 'byGap',
};

export async function servePixels(page: Page) {
  for (const host of ['photo', 'thumb']) {
    await page.route(`http://${host}.localhost/**`, (route) =>
      route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL }),
    );
  }
}

export async function openViewer(page: Page, id: number) {
  await servePixels(page);
  await mockTauri(page, { ...BASE, viewerRequest: { id, scanId: 5, mode: 'series', gap: 10 } });
  await page.setViewportSize({ width: 1200, height: 800 });
  await page.goto('/?window=viewer');
}

export const photo = (page: Page, name: string) => page.getByRole('img', { name });
