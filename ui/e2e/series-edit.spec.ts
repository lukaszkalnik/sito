import { expect, test, type Page } from '@playwright/test';

import { servePixels } from './gallery';
import { invokedCommands, mockTauri, type TauriMock } from './tauri-mock';
import { showView } from './timeline-helpers';

/**
 * Six photos 2 s apart (IMG_00006 is id 0, newest): ids 0–3 one series,
 * ids 4 and 5 single photos.
 */
async function openGrid(page: Page, seriesEdits: TauriMock['seriesEdits']) {
  await servePixels(page);
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [],
    scanResult: { ok: { scanId: 3, photosFound: 6, unreadableEntries: 0, withoutDate: 0 } },
    timeline: { count: 6, newest: '2024-06-01T12:00:00', stepSeconds: 2 },
    series: [{ start: 0, len: 4 }],
    seriesEdits,
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  await showView(page, 'Wszystkie zdjęcia');
}

const tile = (page: Page, name: string) =>
  page.getByTestId('timeline-tile').filter({ hasText: name });
const menu = (page: Page) => page.getByRole('menu', { name: 'Edycja serii' });
const headers = (page: Page) => page.getByRole('heading', { level: 3 });
const pencils = (page: Page) =>
  page.getByRole('img', { name: 'Seria edytowana ręcznie – nie zmienia się z odstępem' });

async function edits(page: Page) {
  return (await invokedCommands(page)).filter((c) => c.cmd === 'edit_series').map((c) => c.args);
}

test('cuts a series into two edited ones at the chosen photo', async ({ page }) => {
  await openGrid(page, [
    {
      series: [
        { start: 0, len: 2, edited: true },
        { start: 2, len: 2, edited: true },
      ],
    },
  ]);
  await expect(headers(page)).toHaveCount(1);

  await tile(page, 'IMG_00004.jpg').click({ button: 'right' });
  await menu(page).getByRole('menuitem', { name: 'Podziel serię przed tym zdjęciem' }).click();

  await expect(page.getByTestId('toast')).toHaveText('Podzielono serię');
  await expect(headers(page)).toHaveCount(2);
  await expect(pencils(page)).toHaveCount(2);
  await expect(menu(page)).toHaveCount(0);
  expect(await edits(page)).toEqual([{ edit: { kind: 'cut', photo: 2 }, maxGapSeconds: 10 }]);
});

test('takes a photo out of its series from the keyboard', async ({ page }) => {
  await openGrid(page, [{ series: [{ start: 0, len: 3, edited: true }], pinned: [3] }]);
  await tile(page, 'IMG_00005.jpg').getByRole('button').focus();

  await page.keyboard.press('Shift+F10');
  await expect(menu(page).getByRole('menuitem').first()).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(menu(page).getByRole('menuitem', { name: 'Wyjmij z serii' })).toBeFocused();
  await page.keyboard.press('Enter');

  await expect(page.getByTestId('toast')).toHaveText('Wyjęto IMG_00005.jpg z serii');
  expect(await edits(page)).toEqual([{ edit: { kind: 'remove', photo: 1 }, maxGapSeconds: 10 }]);
  await expect(tile(page, 'IMG_00005.jpg').getByRole('button')).toBeFocused();
});

test('lets a single photo join the series before it', async ({ page }) => {
  await openGrid(page, [{ series: [{ start: 0, len: 5, edited: true }] }]);

  await tile(page, 'IMG_00002.jpg').click({ button: 'right' });
  await menu(page).getByRole('menuitem', { name: 'Dołącz do poprzedniej serii' }).click();

  await expect(page.getByTestId('toast')).toHaveText('Dołączono IMG_00002.jpg do serii');
  await expect(headers(page)).toHaveText([/^5 zdjęć · 11:59:52–12:00:00/]);
  expect(await edits(page)).toEqual([
    { edit: { kind: 'join', photo: 4, into: 3 }, maxGapSeconds: 10 },
  ]);
});

test('ungroups a series from its header and restores a photo of it', async ({ page }) => {
  await openGrid(page, [
    { series: [], pinned: [0, 1, 2, 3] },
    { series: [], pinned: [0, 1, 2] },
  ]);

  await headers(page).click({ button: 'right' });
  await menu(page).getByRole('menuitem', { name: 'Rozdziel serię' }).click();
  await expect(page.getByTestId('toast')).toHaveText('Rozdzielono serię');
  await expect(headers(page)).toHaveCount(0);
  const pinnedMarks = page.getByRole('img', {
    name: 'Wyjęte z serii ręcznie – nie łączy się w serię z odstępem',
  });
  await expect(pinnedMarks).toHaveCount(4);

  await tile(page, 'IMG_00003.jpg').click({ button: 'right' });
  await menu(page).getByRole('menuitem', { name: 'Przywróć automatyczny podział' }).click();

  await expect(page.getByTestId('toast')).toHaveText('Przywrócono automatyczny podział');
  await expect(pinnedMarks).toHaveCount(3);
  expect(await edits(page)).toEqual([
    { edit: { kind: 'ungroup', photo: 0 }, maxGapSeconds: 10 },
    { edit: { kind: 'restore', photo: 3 }, maxGapSeconds: 10 },
  ]);
});

test('restores an edited series from its header', async ({ page }) => {
  await openGrid(page, [
    { series: [{ start: 0, len: 2, edited: true }], pinned: [2, 3] },
    { series: [{ start: 0, len: 4 }] },
  ]);
  await tile(page, 'IMG_00004.jpg').click({ button: 'right' });
  await menu(page).getByRole('menuitem', { name: 'Wyjmij z serii' }).click();
  await expect(pencils(page)).toHaveCount(1);

  await headers(page).click({ button: 'right' });
  await menu(page).getByRole('menuitem', { name: 'Przywróć automatyczny podział' }).click();

  await expect(pencils(page)).toHaveCount(0);
  await expect(headers(page)).toHaveText(['4 zdjęcia · 11:59:54–12:00:00']);
});

test('closes the menu with Escape without editing, and reports a refused edit', async ({
  page,
}) => {
  await openGrid(page, []);
  await tile(page, 'IMG_00006.jpg').click({ button: 'right' });
  await expect(menu(page)).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(menu(page)).toHaveCount(0);

  await tile(page, 'IMG_00006.jpg').click({ button: 'right' });
  await menu(page).getByRole('menuitem', { name: 'Rozdziel serię' }).click();

  await expect(page.getByTestId('toast')).toHaveText(
    'Nie można zmienić serii – zmieniła się w międzyczasie.',
  );
});
