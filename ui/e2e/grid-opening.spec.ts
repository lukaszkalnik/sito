import { expect, test } from '@playwright/test';

import { mockTauri } from './tauri-mock';

/** Counts the photo tiles removed from the page from its start on, in `window.removedTiles`. */
function recordRemovedTiles() {
  const counter = window as unknown as { removedTiles: number };
  counter.removedTiles = 0;
  const isTile = (node: Node) =>
    node instanceof Element &&
    (node.matches('[data-testid="timeline-tile"]') ||
      node.querySelector('[data-testid="timeline-tile"]') !== null);
  new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      counter.removedTiles += [...mutation.removedNodes].filter(isTile).length;
    }
  }).observe(document, { childList: true, subtree: true });
}

// Rows built for a width the grid did not have yet were replaced right away,
// so a click landing on a tile in that moment was lost (slice 28c).
test('lays out the grid once when a folder opens: no tile is replaced', async ({ page }) => {
  await page.addInitScript(recordRemovedTiles);
  await mockTauri(page, {
    pickedFolder: 'C:\\Zdjęcia',
    scanProgress: [],
    scanResult: { ok: { scanId: 1, photosFound: 30, unreadableEntries: 0, withoutDate: 0 } },
    timeline: { count: 30, newest: '2024-06-01T12:00:00', stepSeconds: 600 },
    series: { every: 10, size: 3 },
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');

  await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();

  await expect(page.getByRole('heading', { level: 3 })).toHaveCount(3);
  await expect(page.getByTestId('timeline-tile')).toHaveCount(9);
  // Two frames more, so a second layout would have landed.
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
  expect(
    await page.evaluate(() => (window as unknown as { removedTiles: number }).removedTiles),
  ).toBe(0);
});
