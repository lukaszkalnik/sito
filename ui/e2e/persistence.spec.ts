import { expect, test, type Page } from '@playwright/test';

import { invokedCommands, mockTauri } from './tauri-mock';
import { photoY, topPhoto } from './timeline-helpers';

const SUMMARY = { scanId: 7, photosFound: 30, unreadableEntries: 0, withoutDate: 0 };

async function start(page: Page, lastFolder: string | null) {
  await page.addInitScript((folder) => {
    if (folder !== null) localStorage.setItem('sito.lastFolder', folder);
  }, lastFolder);
  await mockTauri(page, {
    pickedFolder: 'D:\\Desktop\\Złote gody',
    scanProgress: [],
    // Shown from the cache; the background rescan finds two new photos.
    scanResult: { ok: { ...SUMMARY, refreshing: true } },
    refresh: { summary: { ...SUMMARY, photosFound: 32, unreadableEntries: 1 }, afterMs: 1500 },
    timeline: { count: 30, newest: '2024-06-01T12:00:00', stepSeconds: 1200 },
    series: { every: 10, size: 3 },
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
}

test('reopens the last folder on start and shows the background refresh', async ({ page }) => {
  await start(page, 'D:\\Desktop\\Złote gody');

  await expect(page.getByTestId('timeline')).toBeVisible();
  await expect(page.getByText('Sprawdzanie zmian…')).toBeVisible();
  await expect(page.getByRole('status')).not.toContainText('Sprawdzanie zmian');
  const scans = (await invokedCommands(page)).filter((c) => c.cmd === 'start_scan');
  expect(scans.map((c) => (c.args as { path: string }).path)).toEqual(['D:\\Desktop\\Złote gody']);

  await expect(page.getByRole('status')).toContainText('Nie udało się odczytać 1 elementu.');
  await expect(page.getByText('Sprawdzanie zmian…')).toHaveCount(0);
});

test('starts with the folder picker the first time, then remembers the folder', async ({
  page,
}) => {
  await start(page, null);
  await expect(page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeVisible();

  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  await expect(page.getByTestId('timeline')).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem('sito.lastFolder'))).toBe(
    'D:\\Desktop\\Złote gody',
  );
});

test('reopens a folder in its last view and at its last photo', async ({ page }) => {
  const folder = 'D:\\Desktop\\Złote gody';
  await page.addInitScript((folder) => {
    localStorage.setItem('sito.lastFolder', folder);
    // Seeded once; a reload must find what the app stored itself.
    if (localStorage.getItem('sito.folderView') !== null) return;
    const position = { path: 'Camera\\IMG_00700.jpg', takenAt: '2024-05-31T12:00:00', offset: 0 };
    localStorage.setItem(
      'sito.folderView',
      JSON.stringify({ [folder]: { mode: 'all', position } }),
    );
  }, folder);
  await mockTauri(page, {
    pickedFolder: folder,
    scanProgress: [],
    scanResult: { ok: { ...SUMMARY, photosFound: 1000 } },
    timeline: { count: 1000, newest: '2024-06-01T12:00:00', stepSeconds: 300 },
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');

  await expect(page.getByRole('button', { name: 'Wszystkie zdjęcia' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  // Its row is at the top, wherever it is in the row.
  await expect.poll(() => photoY(page, 'IMG_00700.jpg')).toBe(0);

  // Scroll a bit further and restart.
  await page.getByTestId('timeline').evaluate((el) => el.scrollBy(0, 1000));
  const scrolled = await topPhoto(page);
  expect(await photoY(page, 'IMG_00700.jpg')).not.toBe(0);
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem('sito.folderView')))
    .toContain(scrolled.name);
  await page.reload();

  await expect.poll(() => topPhoto(page)).toEqual(scrolled);
});
