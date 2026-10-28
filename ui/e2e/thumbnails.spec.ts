import { expect, test, type Page } from '@playwright/test';

import { mockTauri } from './tauri-mock';

const PHOTOS = 10000;

// 1×1 grey PNG.
const PIXEL = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNoAAAAggCBd81ytgAAAABJRU5ErkJggg==',
  'base64',
);

/** Serves thumbnails like the backend's `thumb://` protocol; returns the requested URLs. */
async function serveThumbnails(page: Page, failing: ReadonlySet<string> = new Set()) {
  const requested: string[] = [];
  await page.route('http://thumb.localhost/**', (route) => {
    const url = new URL(route.request().url());
    requested.push(url.pathname + url.search);
    return failing.has(url.pathname)
      ? route.fulfill({ status: 500 })
      : route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL });
  });
  return requested;
}

async function openTimeline(page: Page) {
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [],
    scanResult: {
      ok: { scanId: 1700000000000, photosFound: PHOTOS, unreadableEntries: 0, withoutDate: 0 },
    },
    timeline: { count: PHOTOS, newest: '2024-06-01T12:00:00', stepSeconds: 1200 },
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
  await page.getByRole('button', { name: 'Wszystkie zdjęcia' }).click();
  await expect(page.getByTestId('timeline')).toBeVisible();
}

test('loads thumbnails of the visible photos', async ({ page }) => {
  const requested = await serveThumbnails(page);
  await openTimeline(page);

  const firstTile = page.getByTestId('timeline-tile').first();
  await expect(firstTile).toHaveClass(/timeline-tile--loaded/);
  await expect(firstTile.locator('img')).toHaveAttribute(
    'src',
    'http://thumb.localhost/0?scan=1700000000000',
  );
  expect(requested).toContain('/0?scan=1700000000000');
});

test('skips thumbnails of photos that only fly past while scrolling fast', async ({ page }) => {
  const requested = await serveThumbnails(page);
  await openTimeline(page);
  await expect(page.getByTestId('timeline-tile').first()).toHaveClass(/timeline-tile--loaded/);

  // Scroll through the whole timeline in 60 frames, like a fast fling.
  await page.getByTestId('timeline').evaluate(async (el) => {
    const frames = 60;
    for (let i = 1; i <= frames; i++) {
      el.scrollTop = (el.scrollHeight * i) / frames;
      await new Promise(requestAnimationFrame);
    }
  });

  const lastTile = page.getByTestId('timeline-tile').filter({ hasText: 'IMG_00001.jpg' });
  await expect(lastTile).toHaveClass(/timeline-tile--loaded/);
  expect(requested.length).toBeGreaterThan(0);
  expect(requested.length).toBeLessThan(200);
});

test('shows a hint when a thumbnail cannot be loaded', async ({ page }) => {
  await serveThumbnails(page, new Set(['/0']));
  await openTimeline(page);

  const firstTile = page.getByTestId('timeline-tile').first();
  await expect(
    firstTile.getByRole('img', { name: 'Nie udało się wczytać miniatury.' }),
  ).toBeVisible();
  await expect(firstTile).toContainText('IMG_10000.jpg');
  await expect(page.getByTestId('timeline-tile').nth(1)).toHaveClass(/timeline-tile--loaded/);
});
