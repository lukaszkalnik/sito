import type { Page } from '@playwright/test';

/** Name and screen position of the topmost photo visible in the timeline. */
export async function topPhoto(page: Page) {
  return page.getByTestId('timeline').evaluate((timeline) => {
    const top = timeline.getBoundingClientRect().top;
    const tiles = [...timeline.querySelectorAll('[data-testid="timeline-tile"]')]
      .map((tile) => ({ tile, rect: tile.getBoundingClientRect() }))
      .filter(({ rect }) => rect.bottom > top)
      .sort((a, b) => a.rect.top - b.rect.top || a.rect.left - b.rect.left);
    const first = tiles[0];
    return {
      name: first?.tile.querySelector('.timeline-tile-name')?.textContent ?? '',
      y: Math.round((first?.rect.top ?? 0) - top),
    };
  });
}

/** Position of the photo `name` below the top of the timeline, or `null` if not shown. */
export async function photoY(page: Page, name: string) {
  return page.getByTestId('timeline').evaluate((timeline, wanted) => {
    const tile = [...timeline.querySelectorAll('[data-testid="timeline-tile"]')].find(
      (t) => t.querySelector('.timeline-tile-name')?.textContent === wanted,
    );
    return tile
      ? Math.round(tile.getBoundingClientRect().top - timeline.getBoundingClientRect().top)
      : null;
  }, name);
}
