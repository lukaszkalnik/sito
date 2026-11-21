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

test('shows the notes of every new version formatted, without links', async ({ page }) => {
  const notes = [
    '## v0.17.0 – 2026-12-01',
    '',
    '### New',
    '',
    '- **Formatted notes.** With `code`. ([slice 54](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/54-release-notes-markdown.md))',
    '',
    '## v0.16.0 – 2026-11-25',
    '',
    '### Fixed',
    '',
    '- A fix',
  ].join('\n');
  await mockTauri(page, { ...startScreen, update: { version: '0.17.0', notes, install: 'inApp' } });
  await page.goto('/');

  await page
    .getByRole('region', { name: 'Aktualizacja Sito' })
    .getByRole('button', { name: 'Co nowego?' })
    .click();
  const dialog = page.getByRole('dialog', { name: 'Co nowego w wersji 0.17.0' });
  await expect(dialog.getByRole('heading', { level: 3 })).toHaveText([
    'Wersja 0.17.0 · 1 grudnia 2026',
    'Wersja 0.16.0 · 25 listopada 2026',
  ]);
  await expect(dialog.getByRole('heading', { level: 4 })).toHaveText(['New', 'Fixed']);
  await expect(dialog.getByRole('listitem')).toHaveText([
    'Formatted notes. With code. (slice 54)',
    'A fix',
  ]);
  await expect(dialog.locator('strong')).toHaveText('Formatted notes.');
  await expect(dialog.getByRole('link')).toHaveCount(0);
});

test('scrolls long notes with the keyboard, from the top each time', async ({ page }) => {
  const items = Array.from({ length: 60 }, (_, index) => `- Change ${index + 1}`);
  const notes = ['## v0.17.0 – 2026-12-01', '', '### New', '', ...items].join('\n');
  await mockTauri(page, { ...startScreen, update: { version: '0.17.0', notes, install: 'inApp' } });
  await page.goto('/');

  await page
    .getByRole('region', { name: 'Aktualizacja Sito' })
    .getByRole('button', { name: 'Co nowego?' })
    .click();
  const dialog = page.getByRole('dialog', { name: 'Co nowego w wersji 0.17.0' });
  const region = dialog.getByRole('region', { name: 'Opis zmian (po angielsku)' });
  // The notes scroll inside the dialog, which keeps its title and button in view.
  await expect(dialog.getByRole('button', { name: 'Zamknij' })).toBeInViewport();
  await expect(dialog.getByText('Change 60')).not.toBeInViewport();

  await region.focus();
  await page.keyboard.press('End');
  await expect(dialog.getByText('Change 60')).toBeInViewport();

  // Opened again, the notes start at the top.
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Co nowego?' }).click();
  await expect(dialog.getByText('Change 1', { exact: true })).toBeInViewport();
  await expect(dialog.getByText('Change 60')).not.toBeInViewport();
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

test('opens the release page where Sito cannot install the update', async ({ page }) => {
  await mockTauri(page, {
    ...startScreen,
    update: { version: '0.15.0', notes: null, install: 'manual' },
  });
  await page.goto('/');

  const notice = page.getByRole('region', { name: 'Aktualizacja Sito' });
  await expect(notice).toContainText('Pobierz ją ze strony wydań Sito na GitHubie.');
  await expect(notice.getByRole('button', { name: 'Zainstaluj i uruchom ponownie' })).toHaveCount(
    0,
  );

  await notice.getByRole('button', { name: 'Pobierz' }).click();
  await expect
    .poll(async () => (await invokedCommands(page)).some(({ cmd }) => cmd === 'open_release_page'))
    .toBe(true);
  await expect(notice).toContainText('Dostępna jest nowa wersja Sito: 0.15.0.');
});

test('says why the release page did not open', async ({ page }) => {
  await mockTauri(page, {
    ...startScreen,
    update: { version: '0.15.0', notes: null, install: 'manual' },
    openReleasePageError: { code: 'openFailed' },
  });
  await page.goto('/');

  const notice = page.getByRole('region', { name: 'Aktualizacja Sito' });
  await notice.getByRole('button', { name: 'Pobierz' }).click();
  await expect(notice.getByRole('alert')).toHaveText(
    'Nie udało się otworzyć strony wydania. System nie otworzył przeglądarki.',
  );
});
