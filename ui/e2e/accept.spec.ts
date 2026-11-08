import { expect, test } from '@playwright/test';

import { BASE, openViewer, photo, servePixels } from './gallery';
import { mockTauri } from './tauri-mock';

test('accepts the shown photo with A, marks it in the strip and takes it off with A', async ({
  page,
}) => {
  await openViewer(page, 1);
  await expect(photo(page, 'IMG_00029.jpg')).toBeVisible();
  const strip = page.getByRole('navigation', { name: 'Zdjęcia w serii' });

  await page.keyboard.press('a');

  // The viewer stays on the photo.
  await expect(photo(page, 'IMG_00029.jpg')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Cofnij akceptację' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  const toast = page.getByTestId('toast');
  await expect(toast).toContainText('Zaakceptowano IMG_00029.jpg');
  // Accepting is not undone with "Cofnij": the toast has no button and the bar's stays disabled.
  await expect(toast.getByRole('button')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Cofnij', exact: true })).toBeDisabled();
  await expect(strip.getByRole('button', { name: 'IMG_00029.jpg (zaakceptowane)' })).toBeVisible();

  await page.keyboard.press('a');

  await expect(toast).toContainText('Cofnięto akceptację IMG_00029.jpg');
  await expect(toast.getByRole('button')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Zaakceptuj' })).toHaveAttribute(
    'aria-pressed',
    'false',
  );
  await expect(strip.getByRole('img', { name: 'Zaakceptowane' })).toHaveCount(0);
});

test('undo skips accepts and brings back the rejected photo', async ({ page }) => {
  await openViewer(page, 0);
  const strip = page.getByRole('navigation', { name: 'Zdjęcia w serii' });
  await page.getByRole('button', { name: 'Zaakceptuj' }).click();
  await expect(page.getByRole('button', { name: 'Cofnij akceptację' })).toBeVisible();
  await page.keyboard.press('Delete');
  await expect(photo(page, 'IMG_00029.jpg')).toBeVisible();
  await page.keyboard.press('a');
  await expect(page.getByRole('button', { name: 'Cofnij akceptację' })).toBeVisible();

  await page.keyboard.press('Control+z');

  await expect(photo(page, 'IMG_00030.jpg')).toBeVisible();
  // Back from the rejection, still accepted, and the newer accept stays.
  await expect(page.getByRole('button', { name: 'Cofnij akceptację' })).toBeVisible();
  await expect(strip.getByRole('img', { name: 'Zaakceptowane' })).toHaveCount(2);
  await expect(page.getByRole('button', { name: 'Cofnij', exact: true })).toBeDisabled();
});

test('marks accepted photos in the grid, which leave Cofnij disabled', async ({ page }) => {
  await servePixels(page);
  await mockTauri(page, { ...BASE, accepted: [2] });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  const tile = page.getByTestId('timeline-tile').filter({ hasText: 'IMG_00028.jpg' });
  await expect(tile.getByRole('img', { name: 'Zaakceptowane' })).toBeVisible();
  await expect(page.getByRole('img', { name: 'Zaakceptowane' })).toHaveCount(1);

  await expect(page.getByRole('button', { name: 'Cofnij' })).toBeDisabled();
});
