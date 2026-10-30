import { expect, test } from '@playwright/test';

import { BASE, openViewer, photo, servePixels } from './gallery';
import { mockTauri } from './tauri-mock';

test('accepts the shown photo with A, marks it in the strip and undoes it', async ({ page }) => {
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
  await expect(page.getByTestId('toast')).toContainText('Zaakceptowano IMG_00029.jpg');
  await expect(strip.getByRole('button', { name: 'IMG_00029.jpg (zaakceptowane)' })).toBeVisible();

  await page.keyboard.press('Control+z');

  await expect(page.getByTestId('toast')).toContainText('Cofnięto akceptację IMG_00029.jpg');
  await expect(page.getByRole('button', { name: 'Zaakceptuj' })).toHaveAttribute(
    'aria-pressed',
    'false',
  );
  await expect(strip.getByRole('img', { name: 'Zaakceptowane' })).toHaveCount(0);
});

test('undoes accepting and rejecting in one stack, newest first', async ({ page }) => {
  await openViewer(page, 0);
  await page.getByRole('button', { name: 'Zaakceptuj' }).click();
  await expect(page.getByRole('button', { name: 'Cofnij akceptację' })).toBeVisible();
  await page.keyboard.press('Delete');
  await expect(photo(page, 'IMG_00029.jpg')).toBeVisible();

  await page.keyboard.press('Control+z');
  await expect(photo(page, 'IMG_00030.jpg')).toBeVisible();
  // Back from the rejection, still accepted.
  await expect(page.getByRole('button', { name: 'Cofnij akceptację' })).toBeVisible();

  await page.keyboard.press('Control+z');
  await expect(page.getByRole('button', { name: 'Zaakceptuj' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Cofnij', exact: true }).first()).toBeDisabled();
});

test('marks accepted photos in the grid and takes the mark off with Cofnij', async ({ page }) => {
  await servePixels(page);
  await mockTauri(page, { ...BASE, accepted: [2] });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  const tile = page.getByTestId('timeline-tile').filter({ hasText: 'IMG_00028.jpg' });
  await expect(tile.getByRole('img', { name: 'Zaakceptowane' })).toBeVisible();
  await expect(page.getByRole('img', { name: 'Zaakceptowane' })).toHaveCount(1);

  await page.getByRole('button', { name: 'Cofnij' }).click();

  await expect(page.getByTestId('toast')).toContainText('Cofnięto akceptację IMG_00028.jpg');
  await expect(page.getByRole('img', { name: 'Zaakceptowane' })).toHaveCount(0);
});
