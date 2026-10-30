import { expect, test } from '@playwright/test';

import { BASE, servePixels } from './gallery';
import { emitTauriEvent, mockTauri } from './tauri-mock';

/** Opens the main window on the scan of `BASE` (scan 5), showing its series. */
async function openGrid(page: import('@playwright/test').Page) {
  await servePixels(page);
  await mockTauri(page, BASE);
  await page.setViewportSize({ width: 1280, height: 600 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  await expect(page.getByText('IMG_00030.jpg', { exact: true })).toBeVisible();
}

const tile = (page: import('@playwright/test').Page, name: string) =>
  page.getByTestId('timeline-tile').filter({ hasText: name });

test('the grid follows the photo the viewer shows and pulses it once the viewer closes', async ({
  page,
}) => {
  await openGrid(page);
  // Photo 28 (IMG_00002) is near the end of the timeline, out of view.
  await expect(tile(page, 'IMG_00002.jpg')).not.toBeInViewport();

  await emitTauriEvent(page, 'viewer-shown', { scanId: 5, id: 28 });

  const followed = tile(page, 'IMG_00002.jpg');
  await expect(followed).toBeInViewport();
  await expect(followed).toHaveAttribute('aria-current', 'true');
  await expect(followed).not.toHaveClass(/timeline-tile--highlighted/);

  await emitTauriEvent(page, 'viewer-shown', { scanId: 5, id: 27 });
  await expect(tile(page, 'IMG_00003.jpg')).toHaveAttribute('aria-current', 'true');
  await expect(followed).not.toHaveAttribute('aria-current');

  await emitTauriEvent(page, 'viewer-closed');

  const last = tile(page, 'IMG_00003.jpg');
  await expect(last).not.toHaveAttribute('aria-current');
  await expect(last).toHaveClass(/timeline-tile--highlighted-long/);
});

test('ignores photos of another scan', async ({ page }) => {
  await openGrid(page);

  await emitTauriEvent(page, 'viewer-shown', { scanId: 4, id: 0 });

  await expect(tile(page, 'IMG_00030.jpg')).not.toHaveAttribute('aria-current');
});
