import { expect, test, type Page } from '@playwright/test';

import { emitTauriEvent, invokedCommands, mockTauri, type TauriMock } from './tauri-mock';

const SHOW_CACHE_SETTINGS = 'settings-show-cache';
const MB = 1024 * 1024;

const gallery: TauriMock = {
  pickedFolder: 'C:\\Zdjęcia',
  scanProgress: [],
  scanResult: { ok: { scanId: 1, photosFound: 30, unreadableEntries: 0, withoutDate: 0 } },
  timeline: { count: 30, newest: '2024-06-01T12:00:00', stepSeconds: 600 },
  series: { every: 10, size: 3 },
};

/** Picks „Ustawienia → Pamięć podręczna…” once the page listens for it. */
async function pickCacheMenuItem(page: Page): Promise<void> {
  await expect
    .poll(async () =>
      (await invokedCommands(page)).some(
        ({ cmd, args }) =>
          cmd === 'plugin:event|listen' &&
          (args as { event: string }).event === SHOW_CACHE_SETTINGS,
      ),
    )
    .toBe(true);
  await emitTauriEvent(page, SHOW_CACHE_SETTINGS);
}

/** Opens a folder and then the cache settings over its timeline. */
async function openOverTimeline(page: Page, mock: TauriMock = gallery) {
  await mockTauri(page, mock);
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  await expect(page.getByTestId('timeline')).toBeVisible();
  await pickCacheMenuItem(page);
  return page.getByRole('dialog', { name: 'Pamięć podręczna' });
}

test('changes the thumbnail space and empties the thumbnails', async ({ page }) => {
  const dialog = await openOverTimeline(page);
  await expect(dialog).toContainText('Miniatury: 412 MB · Dane zdjęć: 18 MB');

  const limit = dialog.getByLabel('Miejsce na miniatury');
  await limit.fill('20');
  await limit.press('Tab');
  await expect(limit).toHaveValue('50');

  await dialog.getByRole('button', { name: 'Wyczyść miniatury' }).click();
  await dialog.getByRole('alertdialog').getByRole('button', { name: 'Usuń' }).click();
  await expect(dialog).toContainText('Miniatury: 0 kB · Dane zdjęć: 18 MB');

  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
  // Thumbnails already shown stay; new ones are rendered again as needed.
  await expect(page.getByTestId('timeline')).toBeVisible();
  const commands = (await invokedCommands(page)).map(({ cmd }) => cmd);
  expect(commands).toContain('clear_thumbnails');
  expect(commands).not.toContain('clear_photo_data');
});

test('moves the cache to another folder with progress, then back', async ({ page }) => {
  const dialog = await openOverTimeline(page, {
    ...gallery,
    cache: {
      moveProgress: [
        { done: 1000, total: 3000 },
        { done: 3000, total: 3000 },
      ],
      // Long enough for each step to be seen.
      stepMs: 1500,
    },
  });

  await dialog.getByRole('button', { name: 'Zmień…' }).click();
  const question = dialog.getByRole('alertdialog');
  await expect(question).toContainText(
    'Przenieść pamięć podręczną (430 MB) do C:\\Zdjęcia\\Sito-cache?',
  );
  await question.getByRole('button', { name: 'Przenieś' }).click();

  await expect(dialog.getByRole('status')).toHaveText(/Przenoszenie: 1000 z 3000 plików/u);
  await expect(dialog.getByRole('button', { name: 'Zamknij' })).toBeDisabled();
  await expect(dialog.getByRole('button', { name: 'Przywróć domyślny' })).toBeVisible();
  await expect(dialog.getByTitle('C:\\Zdjęcia\\Sito-cache')).toBeVisible();

  await dialog.getByRole('button', { name: 'Przywróć domyślny' }).click();
  await dialog.getByRole('alertdialog').getByRole('button', { name: 'Przenieś' }).click();
  await expect(dialog.getByRole('button', { name: 'Przywróć domyślny' })).toBeHidden();
  const moves = (await invokedCommands(page))
    .filter(({ cmd }) => cmd === 'move_cache')
    .map(({ args }) => (args as { folder: string | null }).folder);
  expect(moves).toEqual(['C:\\Zdjęcia', null]);
});

test('says at start that the chosen cache folder is missing', async ({ page }) => {
  await mockTauri(page, {
    ...gallery,
    cache: {
      settings: {
        folder: 'C:\\Users\\Ala\\AppData\\Local\\io.github.lukaszkalnik.sito',
        location: { kind: 'fallback', chosen: 'D:\\Cache\\Sito-cache' },
        platform: 'windows',
        thumbnailLimitMb: 500,
      },
      size: { thumbnails: 3 * MB, photoData: 0 },
    },
  });
  await page.goto('/');

  const notice = page.getByRole('status').filter({ hasText: 'jest niedostępny' });
  await expect(notice).toHaveText(
    /Folder pamięci podręcznej D:\\Cache\\Sito-cache jest niedostępny – Sito używa folderu domyślnego\./u,
  );
  await notice.getByRole('button', { name: 'Ustawienia…' }).click();

  const dialog = page.getByRole('dialog', { name: 'Pamięć podręczna' });
  await expect(dialog).toContainText('Miniatury: 3 MB · Dane zdjęć: 0 kB');
  await dialog.getByRole('button', { name: 'Przywróć domyślny' }).click();
  await expect(dialog.getByRole('button', { name: 'Przywróć domyślny' })).toBeHidden();
  await dialog.getByRole('button', { name: 'Zamknij' }).click();
  await expect(notice).toBeHidden();
});
