import { expect, test, type Page } from '@playwright/test';

import { mockTauri } from './tauri-mock';

const PHOTOS = 1000;

// Repeating moments of 4 photos: 3 s, 3 s and 8 s apart, then 40 s to the next.
// At 10 s that is 250 series of 4, at 5 s 250 series of 3, at 2 s none.
async function openTimeline(page: Page, folder = 'D:\\Desktop\\Złote gody') {
  await mockTauri(page, {
    pickedFolder: folder,
    scanProgress: [],
    scanResult: {
      ok: { scanId: 1, photosFound: PHOTOS, unreadableEntries: 0, withoutExifDate: 0 },
    },
    timeline: {
      count: PHOTOS,
      newest: '2024-06-01T12:00:00',
      stepSeconds: 0,
      stepPattern: [3, 3, 8, 40],
    },
    series: 'byGap',
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
}

const slider = (page: Page) => page.getByRole('slider', { name: 'Odstęp w serii' });

test('regroups the series when the gap changes', async ({ page }) => {
  await openTimeline(page);
  await expect(slider(page)).toHaveValue('10');
  await expect(page.getByRole('status')).toContainText('250 serii (1000 zdjęć).');

  await slider(page).fill('5');

  await expect(page.getByText('5 s', { exact: true })).toBeVisible();
  await expect(page.getByRole('status')).toContainText('250 serii (750 zdjęć).');
  await expect(page.getByRole('heading', { level: 3 }).first()).toHaveText(
    'Seria · 3 zdjęcia · 11:59:54–12:00:00',
  );

  await slider(page).fill('2');

  await expect(page.getByText('Nie znaleziono serii zdjęć.')).toBeVisible();

  await slider(page).fill('60');

  await expect(page.getByRole('status')).toContainText('1 seria (1000 zdjęć).');
});

test('remembers the gap for the folder', async ({ page }) => {
  await openTimeline(page);
  await slider(page).fill('5');
  await expect(page.getByRole('status')).toContainText('(750 zdjęć)');

  await page.reload();
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  await expect(slider(page)).toHaveValue('5');
  await expect(page.getByRole('status')).toContainText('250 serii (750 zdjęć).');
});

test('starts another folder at the default gap', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('sito.timeGap', JSON.stringify({ 'D:\\Desktop\\Złote gody': 5 }));
  });
  await openTimeline(page, 'D:\\Zdjęcia');

  await expect(slider(page)).toHaveValue('10');
  await expect(page.getByRole('status')).toContainText('250 serii (1000 zdjęć).');
});
