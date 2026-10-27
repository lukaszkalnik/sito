import { expect, test, type Page } from '@playwright/test';

import { mockTauri } from './tauri-mock';

// 1×1 grey PNG, served for photos and thumbnails.
const PIXEL = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNoAAAAggCBd81ytgAAAABJRU5ErkJggg==',
  'base64',
);

// 30 photos (IMG_00030 is id 0); series of 3 at ids 0–2, 10–12 and 20–22.
async function openViewer(page: Page, id: number, mode: 'series' | 'all') {
  for (const host of ['photo', 'thumb']) {
    await page.route(`http://${host}.localhost/**`, (route) =>
      route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL }),
    );
  }
  await mockTauri(page, {
    pickedFolder: null,
    scanProgress: [],
    scanResult: { err: { code: 'internal' } },
    timeline: { count: 30, newest: '2024-06-01T12:00:00', stepSeconds: 1200 },
    series: { every: 10, size: 3 },
    viewerRequest: { id, scanId: 5, mode, gap: 10 },
  });
  await page.setViewportSize({ width: 1200, height: 800 });
  await page.goto('/?window=viewer');
}

const photo = (page: Page, name: string) => page.getByRole('img', { name });

test('shows the photo and its series, and opens photos from the strip', async ({ page }) => {
  await openViewer(page, 1, 'series');

  await expect(photo(page, 'IMG_00029.jpg')).toHaveAttribute(
    'src',
    'http://photo.localhost/1?scan=5',
  );
  const strip = page.getByRole('navigation', { name: 'Zdjęcia w serii' });
  await expect(strip.getByRole('button')).toHaveCount(3);
  await expect(strip.getByRole('button', { name: 'IMG_00029.jpg' })).toHaveAttribute(
    'aria-current',
    'true',
  );

  await strip.getByRole('button', { name: 'IMG_00028.jpg' }).click();

  await expect(photo(page, 'IMG_00028.jpg')).toHaveAttribute(
    'src',
    'http://photo.localhost/2?scan=5',
  );
  await expect(strip.getByRole('button', { name: 'IMG_00028.jpg' })).toHaveAttribute(
    'aria-current',
    'true',
  );
});

test('steps into the next series in the series view', async ({ page }) => {
  await openViewer(page, 2, 'series');
  await expect(photo(page, 'IMG_00028.jpg')).toBeVisible();

  await page.keyboard.press('ArrowRight');

  await expect(photo(page, 'IMG_00020.jpg')).toBeVisible();
  await page.getByRole('button', { name: 'Poprzednie zdjęcie' }).click();
  await expect(photo(page, 'IMG_00028.jpg')).toBeVisible();
});

test('steps to single photos in the all view, without a strip', async ({ page }) => {
  await openViewer(page, 2, 'all');
  await expect(photo(page, 'IMG_00028.jpg')).toBeVisible();

  await page.getByRole('button', { name: 'Następne zdjęcie' }).click();

  await expect(photo(page, 'IMG_00027.jpg')).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Zdjęcia w serii' })).toHaveCount(0);
});
