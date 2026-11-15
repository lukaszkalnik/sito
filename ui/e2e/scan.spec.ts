import { expect, test } from '@playwright/test';

import { emitTauriEvent, invokedCommands, mockTauri } from './tauri-mock';

test('picks a folder, shows both progress phases and then the timeline', async ({ page }) => {
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [
      { phase: 'searching', found: 120 },
      { phase: 'reading', done: 200, total: 1234 },
    ],
    scanResult: { ok: { scanId: 1, photosFound: 1234, unreadableEntries: 0, withoutDate: 3 } },
    timeline: { count: 1234, newest: '2024-06-01T12:00:00', stepSeconds: 600 },
    series: { every: 10, size: 3 },
  });
  await page.goto('/');

  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  await expect(page.getByText('Folder: C:\\Zdjęcia')).toBeVisible();
  await expect(page.getByRole('status')).toContainText('Odczytywanie dat: 200 z 1234');
  await expect(
    page.getByText('Znaleziono 1234 zdjęcia. 124 serie (372 zdjęcia). Bez daty zdjęcia: 3.'),
  ).toBeVisible();
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
    scanResult: { ok: { scanId: 1, photosFound: 0, unreadableEntries: 0, withoutDate: 0 } },
  });
  await page.goto('/');

  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  await expect(page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeEnabled();
  await expect(page.getByRole('status')).toHaveCount(0);
});

test('shows the EXIF dates being completed in the background, then hides it', async ({ page }) => {
  await mockTauri(page, {
    pickedFolder: 'C:\\Takeout',
    scanProgress: [],
    scanResult: { ok: { scanId: 1, photosFound: 50, unreadableEntries: 0, withoutDate: 0 } },
    timeline: { count: 50, newest: '2024-06-01T12:00:00', stepSeconds: 600 },
    completing: {
      steps: [
        { done: 200, total: 34600 },
        { done: 34600, total: 34600 },
      ],
      everyMs: 500,
    },
  });
  await page.goto('/');

  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  const status = page.getByRole('status');
  await expect(page.getByText(/Uzupełnianie dat z EXIF: 200 z 34\s600\./)).toBeVisible();
  await expect(status).not.toContainText('Uzupełnianie dat z EXIF');
  await expect(page.getByText(/Uzupełnianie dat z EXIF/)).toHaveCount(0);
  await expect(status).toContainText('Znaleziono 50 zdjęć.');
});

test('clears the old timeline as soon as another folder is picked', async ({ page }) => {
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [{ phase: 'searching', found: 120 }],
    scanResult: { ok: { scanId: 1, photosFound: 1234, unreadableEntries: 0, withoutDate: 0 } },
    timeline: { count: 1234, newest: '2024-06-01T12:00:00', stepSeconds: 600 },
    series: { every: 10, size: 3 },
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  await expect(page.getByTestId('timeline')).toBeVisible();

  await page.getByRole('button', { name: 'Wybierz inny folder' }).click();

  // The mocked scan takes 300 ms; the old grid must be gone well before.
  await expect(page.getByTestId('timeline')).toBeHidden({ timeout: 150 });
  await expect(page.getByRole('status')).toContainText('Wyszukiwanie zdjęć… znaleziono 120');
});

test('„Odśwież” and F5 check the folder for changes in the background', async ({ page }) => {
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [],
    scanResult: { ok: { scanId: 1, photosFound: 50, unreadableEntries: 0, withoutDate: 0 } },
    timeline: { count: 50, newest: '2024-06-01T12:00:00', stepSeconds: 600 },
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  const refresh = page.getByRole('button', { name: 'Odśwież' });
  await expect(refresh).toBeEnabled();

  await refresh.click();

  await expect(page.getByText('Sprawdzanie zmian…')).toBeVisible();
  await expect(refresh).toBeDisabled();
  await emitTauriEvent(page, 'scan-refreshed', {
    scanId: 1,
    photosFound: 50,
    unreadableEntries: 0,
    withoutDate: 0,
    refreshing: false,
  });
  await expect(page.getByText('Sprawdzanie zmian…')).toHaveCount(0);

  await page.keyboard.press('F5');
  await expect(page.getByText('Sprawdzanie zmian…')).toBeVisible();
  const refreshes = (await invokedCommands(page)).filter((c) => c.cmd === 'refresh_folder');
  expect(refreshes).toHaveLength(2);
});

test('offers a refresh when photos are gone, and another folder when the folder is', async ({
  page,
}) => {
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [],
    scanResult: { ok: { scanId: 1, photosFound: 50, unreadableEntries: 0, withoutDate: 0 } },
    timeline: { count: 50, newest: '2024-06-01T12:00:00', stepSeconds: 600 },
    series: { every: 10, size: 3 },
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  await expect(page.getByTestId('timeline')).toBeVisible();

  await emitTauriEvent(page, 'photos-missing', 1);
  const note = page.getByText(/^Niektórych zdjęć nie ma już w otwartym folderze/);
  await expect(note).toBeVisible();
  await page.getByRole('button', { name: 'Odśwież' }).last().click();
  await expect(note).toHaveCount(0);
  await expect(page.getByText('Sprawdzanie zmian…')).toBeVisible();

  await emitTauriEvent(page, 'folder-missing', 1);
  await expect(page.getByRole('alert')).toContainText('Otwartego folderu nie ma już na dysku');
  await page.getByRole('alert').getByRole('button', { name: 'Wybierz inny folder' }).click();
  await expect(page.getByTestId('timeline')).toBeVisible();
});
