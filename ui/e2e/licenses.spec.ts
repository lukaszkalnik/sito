import { expect, test, type Page } from '@playwright/test';

import { emitTauriEvent, invokedCommands, mockTauri } from './tauri-mock';

const SHOW_LICENSES = 'about-show-licenses';

/** Picks „Pomoc → Licencje…” once the page listens for it. */
async function pickLicensesMenuItem(page: Page): Promise<void> {
  await expect
    .poll(async () =>
      (await invokedCommands(page)).some(
        ({ cmd, args }) =>
          cmd === 'plugin:event|listen' && (args as { event: string }).event === SHOW_LICENSES,
      ),
    )
    .toBe(true);
  await emitTauriEvent(page, SHOW_LICENSES);
}

test.beforeEach(async ({ page }) => {
  // The dev server does not build the notices file.
  await page.route('**/THIRD-PARTY-NOTICES.txt', (route) =>
    route.fulfill({
      contentType: 'text/plain; charset=utf-8',
      body: 'THIRD-PARTY NOTICES\n\nreact 19.3.1 (MIT)\nimage 0.25.10',
    }),
  );
});

test('shows the licenses from the menu over the timeline and closes with Escape', async ({
  page,
}) => {
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [],
    scanResult: { ok: { scanId: 1, photosFound: 30, unreadableEntries: 0, withoutDate: 0 } },
    timeline: { count: 30, newest: '2024-06-01T12:00:00', stepSeconds: 600 },
    series: { every: 10, size: 3 },
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  await expect(page.getByTestId('timeline')).toBeVisible();

  await pickLicensesMenuItem(page);

  const dialog = page.getByRole('dialog', { name: 'Licencje' });
  await expect(dialog).toContainText('Sito 0.9.0 jest dostępne na licencji MIT lub Apache 2.0.');
  await expect(dialog.getByLabel('Treść licencji')).toContainText('react 19.3.1 (MIT)');
  await expect(dialog.getByRole('button', { name: 'Zamknij' })).toBeFocused();

  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
  await expect(page.getByTestId('timeline')).toBeVisible();
});

test('shows the licenses on the start screen and closes with „Zamknij”', async ({ page }) => {
  await mockTauri(page, {
    pickedFolder: null,
    scanProgress: [],
    scanResult: { ok: { scanId: 1, photosFound: 0, unreadableEntries: 0, withoutDate: 0 } },
  });
  await page.goto('/');

  await pickLicensesMenuItem(page);
  const dialog = page.getByRole('dialog', { name: 'Licencje' });
  await expect(dialog).toContainText('image 0.25.10');

  await dialog.getByRole('button', { name: 'Zamknij' }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeVisible();
});
