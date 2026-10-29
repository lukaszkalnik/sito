import { expect, test, type Page } from '@playwright/test';

import { invokedCommands, mockTauri } from './tauri-mock';

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
  // The viewer asks for one photo at a time, never for the whole timeline.
  const commands = (await invokedCommands(page)).map(({ cmd }) => cmd);
  expect(commands).toContain('get_viewer_photo');
  expect(commands).not.toContain('get_timeline');
  expect(commands).not.toContain('get_series');
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

test('says when a date was corrected in Google Photos', async ({ page }) => {
  for (const host of ['photo', 'thumb']) {
    await page.route(`http://${host}.localhost/**`, (route) =>
      route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL }),
    );
  }
  await mockTauri(page, {
    pickedFolder: null,
    scanProgress: [],
    scanResult: { err: { code: 'internal' } },
    timeline: {
      entries: [
        {
          id: 0,
          name: 'skan_babcia.jpg',
          relativeDir: '1976',
          relativePath: '1976\\skan_babcia.jpg',
          takenAt: '1976-07-01T12:00:00',
          timeSource: 'takeoutCorrected',
          format: 'jpeg',
        },
      ],
    },
    viewerRequest: { id: 0, scanId: 5, mode: 'all', gap: 10 },
  });
  await page.goto('/?window=viewer');

  await expect(page.getByText('data poprawiona w Google Photos')).toBeVisible();
  await expect(page.getByText('1 lipca 1976', { exact: false })).toBeVisible();
});

test('keeps the step buttons in place between series and single photos', async ({ page }) => {
  // Photo 1 is in a series (with a strip), photo 3 is a single photo.
  await openViewer(page, 1, 'all');
  const next = page.getByRole('button', { name: 'Następne zdjęcie' });
  const inSeries = await next.boundingBox();

  await next.click();
  await next.click();
  await expect(page.getByRole('navigation', { name: 'Zdjęcia w serii' })).toHaveCount(0);
  const single = await next.boundingBox();

  expect(single?.y).toBe(inSeries?.y);
});

test('shows a spinner while a photo is slow to load', async ({ page }) => {
  await openViewer(page, 1, 'all');
  let release: () => void = () => {};
  const slow = new Promise<void>((resolve) => (release = resolve));
  await page.route('http://photo.localhost/2**', async (route) => {
    await slow;
    await route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL });
  });

  await page.getByRole('button', { name: 'Następne zdjęcie' }).click();

  const spinner = page.getByRole('status', { name: 'Wczytywanie zdjęcia…' });
  await expect(spinner).toBeVisible();
  release();
  await expect(spinner).toHaveCount(0);
});
