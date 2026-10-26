import { expect, test } from '@playwright/test';

import { mockTauri } from './tauri-mock';

test('picks a folder and shows progress and the photo count', async ({ page }) => {
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [
      { entriesSeen: 500, photosFound: 120 },
      { entriesSeen: 1000, photosFound: 230 },
    ],
    scanResult: { ok: { photosFound: 1234, entriesSeen: 2600, unreadableEntries: 0 } },
  });
  await page.goto('/');

  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  await expect(page.getByText('Folder: C:\\Zdjęcia')).toBeVisible();
  await expect(page.getByRole('status')).toContainText('znaleziono 230 zdjęć');
  await expect(page.getByText('Znaleziono 1234 zdjęcia.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Wybierz inny folder' })).toBeEnabled();
});

test('shows a Polish error when the folder is gone', async ({ page }) => {
  await mockTauri(page, {
    pickedFolder: 'C:\\Usunięty',
    scanProgress: [],
    scanResult: { err: { code: 'notFound' } },
  });
  await page.goto('/');

  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  await expect(page.getByRole('alert')).toHaveText('Nie znaleziono wybranego folderu.');
});

test('does nothing when the folder dialog is cancelled', async ({ page }) => {
  await mockTauri(page, {
    pickedFolder: null,
    scanProgress: [],
    scanResult: { ok: { photosFound: 0, entriesSeen: 0, unreadableEntries: 0 } },
  });
  await page.goto('/');

  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  await expect(page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeEnabled();
  await expect(page.getByRole('status')).toHaveCount(0);
});
