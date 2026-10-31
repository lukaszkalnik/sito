import { expect, test, type Page } from '@playwright/test';

import { emitTauriEvent, invokedCommands, mockTauri, type TauriMock } from './tauri-mock';

const CHECK_REQUESTED = 'update-check-requested';

const startScreen: TauriMock = {
  pickedFolder: null,
  scanProgress: [],
  scanResult: { ok: { scanId: 1, photosFound: 0, unreadableEntries: 0, withoutDate: 0 } },
};

/** Picks „Pomoc → Sprawdź aktualizacje…” once the page listens for it. */
async function pickCheckMenuItem(page: Page): Promise<void> {
  await expect
    .poll(async () =>
      (await invokedCommands(page)).some(
        ({ cmd, args }) =>
          cmd === 'plugin:event|listen' && (args as { event: string }).event === CHECK_REQUESTED,
      ),
    )
    .toBe(true);
  await emitTauriEvent(page, CHECK_REQUESTED);
}

test('offers a newer version at the start, shows its notes and installs it', async ({ page }) => {
  await mockTauri(page, {
    ...startScreen,
    update: { version: '0.15.0', notes: '### New\n- Updates in the app', install: 'inApp' },
    installProgress: [{ kind: 'downloading', downloaded: 45, total: 100 }],
  });
  await page.goto('/');

  const notice = page.getByRole('region', { name: 'Aktualizacja Sito' });
  await expect(notice).toContainText('Dostępna jest nowa wersja Sito: 0.15.0.');

  await notice.getByRole('button', { name: 'Co nowego?' }).click();
  const dialog = page.getByRole('dialog', { name: 'Co nowego w wersji 0.15.0' });
  await expect(dialog).toContainText('Updates in the app');
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();

  await notice.getByRole('button', { name: 'Zainstaluj i uruchom ponownie' }).click();
  await expect(notice).toContainText('Pobieranie aktualizacji… 45%');
  // The start screen stays usable next to the notice.
  await expect(page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeEnabled();
});

test('shows nothing at the start when this is the newest version', async ({ page }) => {
  await mockTauri(page, startScreen);
  await page.goto('/');

  await expect
    .poll(async () => (await invokedCommands(page)).some(({ cmd }) => cmd === 'check_for_update'))
    .toBe(true);
  await expect(page.getByRole('region', { name: 'Aktualizacja Sito' })).toBeEmpty();
});

test('says from the menu that this is the newest version', async ({ page }) => {
  await mockTauri(page, startScreen);
  await page.goto('/');

  await pickCheckMenuItem(page);

  const notice = page.getByRole('region', { name: 'Aktualizacja Sito' });
  await expect(notice).toContainText('Masz najnowszą wersję (0.9.0).');
  await notice.getByRole('button', { name: 'Zamknij' }).click();
  await expect(notice).toBeEmpty();
});
