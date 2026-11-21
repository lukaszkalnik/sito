import { expect, test, type Page } from '@playwright/test';

import { BASE, servePixels } from './gallery';
import { emitTauriEvent, invokedCommands, mockTauri } from './tauri-mock';

/** The texts each walk looks for, per language. */
const LANGUAGES = [
  {
    info: { language: 'en', locale: 'en-GB', choice: 'en' },
    chooseFolder: 'Choose a photo folder',
    summary: '30 photos · 10 series',
    day: 'Saturday, 1 June 2024',
    view: 'View',
    all: 'All photos',
    destination: 'Rejected photos to:',
    trash: 'Recycle Bin',
    confirm: 'Move rejected photos to the Recycle Bin?',
    cancel: 'Cancel',
    menu: 'Series editing',
    licenses: 'Licenses',
    update: 'Sito update',
    whatsNew: "What's new?",
    reject: 'Reject',
    accept: 'Accept',
    unaccept: 'Unaccept',
    rejected: 'IMG_00029.jpg → Rejected',
    undo: 'Undo',
  },
  {
    info: { language: 'de', locale: 'de-DE', choice: 'system' },
    chooseFolder: 'Fotoordner wählen',
    summary: '30 Fotos · 10 Serien',
    day: 'Samstag, 1. Juni 2024',
    view: 'Ansicht',
    all: 'Alle Fotos',
    destination: 'Abgelehnte Fotos in:',
    trash: 'Papierkorb',
    confirm: 'Abgelehnte Fotos in den Papierkorb verschieben?',
    cancel: 'Abbrechen',
    menu: 'Serie bearbeiten',
    licenses: 'Lizenzen',
    update: 'Sito-Update',
    whatsNew: 'Was ist neu?',
    reject: 'Ablehnen',
    accept: 'Akzeptieren',
    unaccept: 'Zurücknehmen',
    rejected: 'IMG_00029.jpg → Abgelehnt',
    undo: 'Rückgängig',
  },
] as const;

/** Collects the texts i18next reports as missing (logged in development builds). */
function missingTexts(page: Page): string[] {
  const missing: string[] = [];
  page.on('console', (message) => {
    if (message.text().startsWith('missing translation')) missing.push(message.text());
  });
  return missing;
}

/** Sends `event` once the page listens for it, like a click on a native menu item. */
async function emitWhenListening(page: Page, event: string, payload: unknown = null) {
  await expect
    .poll(async () =>
      (await invokedCommands(page)).some(
        ({ cmd, args }) =>
          cmd === 'plugin:event|listen' && (args as { event: string }).event === event,
      ),
    )
    .toBe(true);
  await emitTauriEvent(page, event, payload);
}

for (const texts of LANGUAGES) {
  const { language, locale } = texts.info;

  test.describe(`in ${language}`, () => {
    test.beforeEach(async ({ page }) => {
      await servePixels(page);
      await page.route('**/THIRD-PARTY-NOTICES.txt', (route) =>
        route.fulfill({ contentType: 'text/plain; charset=utf-8', body: 'react 19.3.1 (MIT)' }),
      );
    });

    test('walks the main window without a missing text', async ({ page }) => {
      const missing = missingTexts(page);
      await mockTauri(page, {
        ...BASE,
        language: texts.info,
        update: { version: '0.15.0', notes: 'Notes', install: 'inApp' },
      });
      await page.setViewportSize({ width: 1280, height: 800 });
      await page.goto('/');
      await expect(page.locator('html')).toHaveAttribute('lang', locale);

      const notice = page.getByRole('region', { name: texts.update });
      await notice.getByRole('button', { name: texts.whatsNew }).click();
      await page.keyboard.press('Escape');

      await page.getByRole('button', { name: texts.chooseFolder }).click();
      await expect(page.getByText(texts.summary, { exact: true })).toBeVisible();
      await expect(page.getByRole('heading', { name: texts.day })).toBeVisible();

      await page.getByRole('combobox', { name: texts.view }).selectOption({ label: texts.all });
      await page.getByTestId('timeline-tile').first().click({ button: 'right' });
      await expect(page.getByRole('menu', { name: texts.menu })).toBeVisible();
      await page.keyboard.press('Escape');

      await page.getByRole('combobox', { name: texts.destination }).selectOption({
        label: texts.trash,
      });
      const confirm = page.getByRole('dialog', { name: texts.confirm });
      await confirm.getByRole('button', { name: texts.cancel }).click();
      await expect(confirm).toBeHidden();

      await emitWhenListening(page, 'about-show-licenses');
      await expect(page.getByRole('dialog', { name: texts.licenses })).toContainText('react');
      await page.keyboard.press('Escape');

      expect(missing).toEqual([]);
    });

    test('walks the viewer without a missing text', async ({ page }) => {
      const missing = missingTexts(page);
      await mockTauri(page, {
        ...BASE,
        language: texts.info,
        viewerRequest: { id: 1, scanId: 5, mode: 'series', gap: 10 },
      });
      await page.setViewportSize({ width: 1200, height: 800 });
      await page.goto('/?window=viewer');
      await expect(page.locator('html')).toHaveAttribute('lang', locale);

      await page.getByRole('button', { name: texts.accept, exact: true }).click();
      await expect(page.getByRole('button', { name: texts.unaccept })).toBeVisible();
      await page.getByRole('button', { name: texts.reject, exact: true }).click();
      const toast = page.getByTestId('toast');
      await expect(toast).toContainText(texts.rejected);
      await toast.getByRole('button', { name: texts.undo }).click();

      expect(missing).toEqual([]);
    });
  });
}

test('follows a language change in an open window', async ({ page }) => {
  await mockTauri(page, BASE);
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl-PL');

  await emitWhenListening(page, 'language-changed', {
    language: 'de',
    locale: 'de-AT',
    choice: 'de',
  });

  await expect(page.getByRole('button', { name: 'Fotoordner wählen' })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', 'de-AT');
});

test('stays Polish when the backend cannot say the language', async ({ page }) => {
  await mockTauri(page, {
    ...BASE,
    language: { language: 'fr', locale: 'fr-FR', choice: 'fr' } as never,
  });
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeVisible();
});
