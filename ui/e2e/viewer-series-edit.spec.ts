import { expect, test, type Page } from '@playwright/test';

import { invokedCommands, mockTauri, type TauriMock } from './tauri-mock';

// 1×1 grey PNG, served for photos and thumbnails.
const PIXEL = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNoAAAAggCBd81ytgAAAABJRU5ErkJggg==',
  'base64',
);

/**
 * The viewer on photo `id` of 30 photos (IMG_00030 is id 0) whose ids 0–3
 * are a series and 10–12 another.
 */
async function openViewer(
  page: Page,
  id: number,
  seriesEdits: TauriMock['seriesEdits'],
  mode: 'series' | 'all' = 'all',
) {
  for (const protocol of ['photo', 'thumb']) {
    await page.route(`http://${protocol}.localhost/**`, (route) =>
      route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL }),
    );
  }
  await mockTauri(page, {
    pickedFolder: null,
    scanProgress: [],
    scanResult: { err: { code: 'internal' } },
    timeline: { count: 30, newest: '2024-06-01T12:00:00', stepSeconds: 1200 },
    series: [
      { start: 0, len: 4 },
      { start: 10, len: 3 },
    ],
    seriesEdits,
    viewerRequest: { id, scanId: 5, mode, gap: 10 },
  });
  await page.setViewportSize({ width: 1200, height: 800 });
  await page.goto('/?window=viewer');
}

const name = (id: number) => `IMG_${String(30 - id).padStart(5, '0')}.jpg`;
const shown = (page: Page, id: number) => page.getByRole('img', { name: name(id) });
const strip = (page: Page) => page.getByRole('navigation', { name: 'Zdjęcia w serii' });
const thumb = (page: Page, id: number) => strip(page).getByRole('button', { name: name(id) });
const menu = (page: Page) => page.getByRole('menu', { name: 'Edycja serii' });
const editButton = (page: Page) => page.getByRole('button', { name: 'Edytuj serię' });
const toast = (page: Page) => page.getByTestId('toast');
const edited = (page: Page) =>
  strip(page).getByRole('img', { name: 'Seria edytowana ręcznie – nie zmienia się z odstępem' });

async function edits(page: Page) {
  return (await invokedCommands(page)).filter((c) => c.cmd === 'edit_series').map((c) => c.args);
}

for (const mode of ['all', 'series'] as const) {
  test(`cuts and takes out the shown photo with P and W (${mode})`, async ({ page }) => {
    await openViewer(
      page,
      1,
      [
        // P: id 0 on its own, ids 1–3 an edited series.
        { series: [{ start: 1, len: 3, edited: true }], pinned: [0] },
        // W: id 1 taken out, ids 2–3 left.
        { series: [{ start: 2, len: 2, edited: true }], pinned: [0, 1] },
      ],
      mode,
    );
    await expect(strip(page).getByRole('button')).toHaveCount(4);

    await page.keyboard.press('p');
    await expect(toast(page)).toContainText('Podzielono serię');
    await expect(strip(page).getByRole('button')).toHaveCount(3);
    await expect(edited(page)).toBeVisible();
    await expect(shown(page, 1)).toBeVisible();

    await page.keyboard.press('w');
    await expect(toast(page)).toContainText(`Wyjęto ${name(1)} z serii`);
    // The viewer stays in the series, on the photo after the one taken out.
    await expect(shown(page, 2)).toBeVisible();
    await expect(strip(page).getByRole('button')).toHaveCount(2);
    expect(await edits(page)).toEqual([
      { edit: { kind: 'cut', photo: 1 }, maxGapSeconds: 10 },
      { edit: { kind: 'remove', photo: 1 }, maxGapSeconds: 10 },
    ]);
  });
}

test('edits from a strip thumbnail, keeping the shown photo', async ({ page }) => {
  await openViewer(page, 0, [
    {
      series: [
        { start: 0, len: 2, edited: true },
        { start: 2, len: 2, edited: true },
      ],
    },
    { series: [{ start: 2, len: 2, edited: true }], pinned: [0, 1] },
  ]);

  await thumb(page, 2).click({ button: 'right' });
  await menu(page).getByRole('menuitem', { name: 'Podziel serię przed tym zdjęciem' }).click();
  await expect(toast(page)).toContainText('Podzielono serię');
  await expect(strip(page).getByRole('button')).toHaveCount(2);
  await expect(shown(page, 0)).toBeVisible();

  // An edited series of two offers only ungrouping and restoring.
  await thumb(page, 1).click({ button: 'right' });
  await expect(menu(page).getByRole('menuitem')).toHaveText([
    'Rozdziel serię',
    'Przywróć automatyczne grupowanie',
  ]);
  await menu(page).getByRole('menuitem', { name: 'Rozdziel serię' }).click();

  await expect(toast(page)).toContainText('Rozdzielono serię');
  await expect(strip(page)).toHaveCount(0);
  await expect(shown(page, 0)).toBeVisible();
  await expect(page.getByRole('img', { name: /Wyjęte z serii ręcznie/ })).toBeVisible();
});

test('restores a photo taken out and undoes the restore', async ({ page }) => {
  await openViewer(page, 3, [
    { series: [{ start: 0, len: 3, edited: true }], pinned: [3] },
    { series: [{ start: 0, len: 4 }], changed: [3] },
  ]);

  await expect(shown(page, 3)).toBeVisible();

  // The last photo of the series: the viewer goes back to the one before it.
  await page.keyboard.press('w');
  await expect(shown(page, 2)).toBeVisible();
  await page.keyboard.press('ArrowRight');
  await expect(shown(page, 3)).toBeVisible();
  await expect(strip(page)).toHaveCount(0);
  await expect(page.getByRole('img', { name: /Wyjęte z serii ręcznie/ })).toBeVisible();

  await editButton(page).click();
  await expect(menu(page).getByRole('menuitem')).toHaveText(['Przywróć automatyczne grupowanie']);
  await menu(page).getByRole('menuitem').click();

  await expect(toast(page)).toContainText('Przywrócono automatyczne grupowanie');
  await expect(strip(page).getByRole('button')).toHaveCount(4);
  await expect(thumb(page, 3)).toHaveClass(/viewer-strip-item--pulsed/);
  await expect(thumb(page, 2)).not.toHaveClass(/viewer-strip-item--pulsed/);

  await page.keyboard.press('Control+z');
  await expect(toast(page)).toHaveText('Cofnięto przywrócenie automatycznego grupowania');
  await expect(strip(page)).toHaveCount(0);
  await expect(shown(page, 3)).toBeVisible();
});

test('Escape closes only the menu, and the menu keys work', async ({ page }) => {
  await openViewer(page, 1, [{ series: [{ start: 0, len: 4 }] }]);
  await expect(shown(page, 1)).toBeVisible();

  await page.keyboard.press('Shift+F10');
  await expect(menu(page).getByRole('menuitem').first()).toBeFocused();
  for (const key of ['ArrowRight', 'Delete', 'p', 'w']) await page.keyboard.press(key);
  await page.keyboard.press('Escape');

  await expect(menu(page)).toHaveCount(0);
  await expect(shown(page, 1)).toBeVisible();
  await expect(editButton(page)).toBeFocused();
  expect(await edits(page)).toEqual([]);
  const commands = (await invokedCommands(page)).map((c) => c.cmd);
  expect(commands).not.toContain('reject_photo');
});

test('the menu is dark in the viewer also with a light system theme', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' });
  await openViewer(page, 1, []);

  await editButton(page).click();

  const item = menu(page).getByRole('menuitem', { name: 'Wyjmij z serii' });
  await expect(item).toHaveCSS('color', 'rgb(243, 244, 246)');
  await expect(menu(page)).toHaveCSS('background-color', 'rgb(22, 23, 29)');
});
