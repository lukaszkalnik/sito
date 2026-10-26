import { expect, test } from '@playwright/test';

import { mockTauri } from './tauri-mock';

test('picks a folder, shows both progress phases and then the timeline', async ({ page }) => {
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [
      { phase: 'searching', found: 120 },
      { phase: 'reading', done: 200, total: 1234 },
    ],
    scanResult: { ok: { photosFound: 1234, unreadableEntries: 0, withoutExifDate: 3 } },
    timeline: { count: 1234, newest: '2024-06-01T12:00:00', stepSeconds: 600 },
  });
  await page.goto('/');

  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  await expect(page.getByText('Folder: C:\\Zdjęcia')).toBeVisible();
  await expect(page.getByRole('status')).toContainText('Odczytywanie dat: 200 z 1234');
  await expect(page.getByText('Znaleziono 1234 zdjęcia. Bez daty EXIF: 3.')).toBeVisible();
  await expect(page.getByTestId('timeline')).toBeVisible();
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
    scanResult: { ok: { photosFound: 0, unreadableEntries: 0, withoutExifDate: 0 } },
  });
  await page.goto('/');

  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  await expect(page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeEnabled();
  await expect(page.getByRole('status')).toHaveCount(0);
});
