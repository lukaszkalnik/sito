import { expect, test, type Page } from '@playwright/test';

import { mockTauri, type TauriMock } from './tauri-mock';

// 1×1 grey PNG, served for photos and thumbnails.
const PIXEL = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNoAAAAggCBd81ytgAAAABJRU5ErkJggg==',
  'base64',
);

// 30 photos (IMG_00030 is id 0) in moments of 3 shot 2 s apart, 10 minutes
// between moments: series at ids 0–2, 3–5 and so on, regrouped by the gap
// like the backend does when a photo is rejected.
const BASE: TauriMock = {
  pickedFolder: 'C:\\Zdjęcia',
  scanProgress: [],
  scanResult: { ok: { scanId: 5, photosFound: 30, unreadableEntries: 0, withoutDate: 0 } },
  timeline: { count: 30, newest: '2024-06-01T12:00:00', stepSeconds: 0, stepPattern: [2, 2, 600] },
  series: 'byGap',
};

async function servePixels(page: Page) {
  for (const host of ['photo', 'thumb']) {
    await page.route(`http://${host}.localhost/**`, (route) =>
      route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL }),
    );
  }
}

async function openViewer(page: Page, id: number) {
  await servePixels(page);
  await mockTauri(page, { ...BASE, viewerRequest: { id, scanId: 5, mode: 'series', gap: 10 } });
  await page.setViewportSize({ width: 1200, height: 800 });
  await page.goto('/?window=viewer');
}

const photo = (page: Page, name: string) => page.getByRole('img', { name });

test('rejects the shown photo, continues with the next one and undoes from the toast', async ({
  page,
}) => {
  await openViewer(page, 1);
  await expect(photo(page, 'IMG_00029.jpg')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Cofnij' })).toBeDisabled();

  await page.keyboard.press('Delete');

  await expect(photo(page, 'IMG_00028.jpg')).toBeVisible();
  const toast = page.getByTestId('toast');
  await expect(toast).toContainText('IMG_00029.jpg → Odrzucone');
  // The series strip no longer has the rejected photo.
  const strip = page.getByRole('navigation', { name: 'Zdjęcia w serii' });
  await expect(strip.getByRole('button')).toHaveCount(2);

  await toast.getByRole('button', { name: 'Cofnij' }).click();

  await expect(photo(page, 'IMG_00029.jpg')).toBeVisible();
  await expect(page.getByTestId('toast')).toContainText('Przywrócono IMG_00029.jpg');
  await expect(strip.getByRole('button')).toHaveCount(3);
});

test('undoes several rejections with Ctrl+Z, newest first', async ({ page }) => {
  await openViewer(page, 0);
  await page.getByRole('button', { name: 'Odrzuć' }).click();
  await expect(photo(page, 'IMG_00029.jpg')).toBeVisible();
  await page.getByRole('button', { name: 'Odrzuć' }).click();
  await expect(photo(page, 'IMG_00028.jpg')).toBeVisible();

  await page.keyboard.press('Control+z');
  await expect(photo(page, 'IMG_00029.jpg')).toBeVisible();
  await page.keyboard.press('Control+z');
  await expect(photo(page, 'IMG_00030.jpg')).toBeVisible();

  await expect(page.getByRole('button', { name: 'Cofnij' }).first()).toBeDisabled();
});

test('undoes a rejection from the main window', async ({ page }) => {
  await servePixels(page);
  await mockTauri(page, { ...BASE, rejected: [1] });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  await page.getByRole('button', { name: 'Wszystkie zdjęcia' }).click();
  await expect(page.getByText('IMG_00030.jpg', { exact: true })).toBeVisible();
  await expect(page.getByText('IMG_00029.jpg', { exact: true })).toHaveCount(0);
  await expect(page.getByRole('status')).toContainText('Znaleziono 29 zdjęć.');

  await page.getByRole('button', { name: 'Cofnij' }).click();

  await expect(page.getByText('IMG_00029.jpg', { exact: true })).toBeVisible();
  await expect(page.getByTestId('toast')).toContainText('Przywrócono IMG_00029.jpg');
  await expect(page.getByRole('status')).toContainText('Znaleziono 30 zdjęć.');
  await expect(page.getByRole('button', { name: 'Cofnij' })).toBeDisabled();
});

test('scrolls to and highlights a photo restored from the main window', async ({ page }) => {
  await servePixels(page);
  // The rejected photo is near the end of the timeline, out of view.
  await mockTauri(page, { ...BASE, rejected: [28] });
  await page.setViewportSize({ width: 1280, height: 600 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  await expect(page.getByText('IMG_00030.jpg', { exact: true })).toBeVisible();
  const restored = page.getByTestId('timeline-tile').filter({ hasText: 'IMG_00002.jpg' });
  await expect(restored).toHaveCount(0);

  await page.getByRole('button', { name: 'Cofnij' }).click();

  await expect(restored).toBeInViewport();
  await expect(restored).toHaveClass(/timeline-tile--highlighted/);
});

test('keeps a series of two as a series when one of its photos is rejected', async ({ page }) => {
  // Pairs of photos 2 s apart, 10 minutes between pairs: series at ids 0–1, 2–3, …
  await servePixels(page);
  await mockTauri(page, {
    ...BASE,
    timeline: { count: 30, newest: '2024-06-01T12:00:00', stepSeconds: 0, stepPattern: [2, 600] },
    viewerRequest: { id: 1, scanId: 5, mode: 'series', gap: 10 },
  });
  await page.setViewportSize({ width: 1200, height: 800 });
  await page.goto('/?window=viewer');
  await expect(photo(page, 'IMG_00029.jpg')).toBeVisible();

  await page.keyboard.press('Delete');

  // Stays in the series with the kept photo instead of jumping to the next pair.
  await expect(photo(page, 'IMG_00030.jpg')).toBeVisible();
  const strip = page.getByRole('navigation', { name: 'Zdjęcia w serii' });
  await expect(strip.getByRole('button')).toHaveCount(1);
});

test('shows a series reduced to one photo in the series view', async ({ page }) => {
  await servePixels(page);
  await mockTauri(page, {
    ...BASE,
    timeline: { count: 30, newest: '2024-06-01T12:00:00', stepSeconds: 0, stepPattern: [2, 600] },
    rejected: [1],
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  await expect(page.getByRole('heading', { level: 3 }).first()).toHaveText('1 zdjęcie · 12:00:00');
  await expect(page.getByRole('status')).toContainText('15 serii (29 zdjęć).');
});
