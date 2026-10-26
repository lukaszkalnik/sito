import { expect, test } from '@playwright/test';

test.describe('before the app script runs', () => {
  test.use({ javaScriptEnabled: false });

  test('shows a loading spinner instead of a blank page', async ({ page }) => {
    await page.goto('/');

    await expect(page.getByTestId('boot-spinner')).toBeVisible();
  });
});

test('replaces the spinner once the app has loaded', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'Sito' })).toBeVisible();
  await expect(page.getByTestId('boot-spinner')).toHaveCount(0);
});
