import { expect, test } from '@playwright/test';

test('shows the start screen in Polish', async ({ page }) => {
  await page.goto('/');

  await expect(page).toHaveTitle('Sito');
  await expect(page.getByRole('heading', { name: 'Sito' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeVisible();
});
