import { expect, test, type Page } from '@playwright/test';

import { invokedCommands, mockTauri } from './tauri-mock';

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
