import { expect, test } from '@playwright/test';

import { mockTauri } from './tauri-mock';

const PHOTOS = 10000;

test.beforeEach(async ({ page }) => {
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [],
    scanResult: {
      ok: { scanId: 1, photosFound: PHOTOS, unreadableEntries: 0, withoutExifDate: 0 },
    },
    // One photo every 20 minutes, newest on Saturday 1 June 2024 at noon:
    // 10 000 photos span about 139 days, back to mid-January 2024.
    timeline: { count: PHOTOS, newest: '2024-06-01T12:00:00', stepSeconds: 1200, fileDateEvery: 7 },
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  await expect(page.getByTestId('timeline')).toBeVisible();
});

test('starts with the newest day in Polish', async ({ page }) => {
  const firstHeader = page.getByRole('heading', { level: 2 }).first();

  await expect(firstHeader).toHaveText('sobota, 1 czerwca 2024');
  await expect(page.getByTestId('timeline-tile').first()).toContainText('12:00:00');
});

test('shows the path relative to the scanned folder on hover', async ({ page }) => {
  await expect(page.getByTestId('timeline-tile').first()).toHaveAttribute(
    'title',
    'Camera\\IMG_10000.jpg',
  );
});

test('renders only the visible part of 10 000 photos', async ({ page }) => {
  await expect(page.getByTestId('timeline-tile').first()).toBeVisible();

  const rendered = await page.getByTestId('timeline-tile').count();
  expect(rendered).toBeGreaterThan(0);
  expect(rendered).toBeLessThan(200);
});

test('scrolls to the oldest photo at the end', async ({ page }) => {
  const timeline = page.getByTestId('timeline');

  await timeline.evaluate((el) => el.scrollTo(0, el.scrollHeight));

  await expect(page.getByText('IMG_00001.jpg')).toBeVisible();
  await expect(page.getByRole('heading', { level: 2 }).last()).toContainText('stycznia 2024');
});

test('marks photos dated by the file instead of EXIF', async ({ page }) => {
  await expect(page.getByText('data pliku').first()).toBeVisible();
});
