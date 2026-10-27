import { expect, test, type Page } from '@playwright/test';

import { mockTauri, type TauriMock } from './tauri-mock';

const PHOTOS = 10000;

async function openTimeline(page: Page, series: TauriMock['series']) {
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [],
    scanResult: {
      ok: { scanId: 1, photosFound: PHOTOS, unreadableEntries: 0, withoutExifDate: 0 },
    },
    // One photo every 20 minutes, newest on Saturday 1 June 2024 at noon.
    timeline: { count: PHOTOS, newest: '2024-06-01T12:00:00', stepSeconds: 1200 },
    series,
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
}

// A series of 3 photos starting at every 10th photo: 1000 series, 3000 photos.
const EVERY_TENTH = { every: 10, size: 3 };

test('shows only series by default, with their count', async ({ page }) => {
  await openTimeline(page, EVERY_TENTH);

  await expect(page.getByRole('status')).toContainText('1000 serii (3000 zdjęć).');
  await expect(page.getByRole('button', { name: 'Serie' })).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByRole('heading', { level: 3 }).first()).toHaveText(
    'Seria · 3 zdjęcia · 11:20:00–12:00:00',
  );
  await expect(page.getByText('IMG_10000.jpg')).toBeVisible();
  await expect(page.getByText('IMG_09990.jpg')).toBeVisible();
  // The 4th photo is not part of a series, so it is hidden.
  await expect(page.getByText('IMG_09997.jpg')).toHaveCount(0);
});

test('marks series inside the full timeline', async ({ page }) => {
  await openTimeline(page, EVERY_TENTH);

  await page.getByRole('button', { name: 'Wszystkie zdjęcia' }).click();

  await expect(page.getByText('IMG_09997.jpg')).toBeVisible();
  await expect(page.getByRole('heading', { level: 3 }).first()).toHaveText(
    'Seria · 3 zdjęcia · 11:20:00–12:00:00',
  );
  const firstSeriesTile = page.getByTestId('timeline-tile').filter({ hasText: 'IMG_10000.jpg' });
  await expect(firstSeriesTile.locator('xpath=ancestor::div[1]')).toHaveClass(
    /timeline-row--series/,
  );
  const singleTile = page.getByTestId('timeline-tile').filter({ hasText: 'IMG_09997.jpg' });
  await expect(singleTile.locator('xpath=ancestor::div[1]')).not.toHaveClass(
    /timeline-row--series/,
  );
});

test('renders only the visible series and reaches the oldest one', async ({ page }) => {
  await openTimeline(page, EVERY_TENTH);
  const timeline = page.getByTestId('timeline');
  await expect(timeline).toBeVisible();

  expect(await page.getByTestId('timeline-tile').count()).toBeLessThan(200);
  await timeline.evaluate((el) => el.scrollTo(0, el.scrollHeight));

  // The last series starts at photo 9990 (IMG_00010) and ends with IMG_00008.
  await expect(page.getByText('IMG_00008.jpg')).toBeVisible();
  expect(await page.getByTestId('timeline-tile').count()).toBeLessThan(200);
});

test('explains when there are no series', async ({ page }) => {
  await openTimeline(page, []);

  await expect(page.getByText('Nie znaleziono serii zdjęć.')).toBeVisible();
  await expect(page.getByRole('status')).toContainText('0 serii (0 zdjęć).');

  await page.getByRole('button', { name: 'Wszystkie zdjęcia' }).click();

  await expect(page.getByTestId('timeline')).toBeVisible();
});
