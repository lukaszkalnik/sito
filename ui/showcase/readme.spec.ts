import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { expect, test, type Page } from '@playwright/test';

import { mockTauri, type TauriMock } from '../e2e/tauri-mock';

// Built by scripts/make-demo-gallery.py.
const DEMO = fileURLToPath(new URL('../../demo/', import.meta.url));
const IMAGES = fileURLToPath(new URL('../../docs/images/', import.meta.url));
const MANIFEST = `${DEMO}manifest.json`;

interface Entry {
  id: number;
  name: string;
  relativeDir: string;
}

const entries: Entry[] = existsSync(MANIFEST)
  ? (JSON.parse(readFileSync(MANIFEST, 'utf8')) as Entry[])
  : [];

/** The demo photo with this file name. */
function entry(name: string): Entry {
  const found = entries.find((e) => e.name === name);
  if (!found) throw new Error(`${name} is not in the demo gallery`);
  return found;
}

const SCAN_ID = 1;
const SIZE = { width: 1280, height: 800 };
// JPEG keeps the photo-heavy screenshots small enough for the README.
const SHOT = { type: 'jpeg', quality: 88, animations: 'disabled' } as const;
// The dog series: four shots seconds apart, the last one blurred.
const DOG_SHARP = '20240615_092004.jpg';
const DOG_BLURRED = '20240615_092009.jpg';

/** Serves `thumb://` and `photo://` from the demo gallery, like the backend does. */
async function serveDemo(page: Page): Promise<void> {
  const fileOf = (url: string, host: 'thumb' | 'photo') => {
    const id = Number(new URL(url).pathname.slice(1));
    const e = entries.find((x) => x.id === id);
    if (!e) return undefined;
    return host === 'thumb'
      ? `${DEMO}thumbs/${id}.jpg`
      : `${DEMO}gallery/${e.relativeDir}/${e.name}`;
  };
  for (const host of ['thumb', 'photo'] as const) {
    await page.route(`http://${host}.localhost/**`, (route) => {
      const path = fileOf(route.request().url(), host);
      return path ? route.fulfill({ path }) : route.fulfill({ status: 404 });
    });
  }
}

function demoMock(extra: Partial<TauriMock> = {}): TauriMock {
  return {
    pickedFolder: 'D:\\Zdjęcia\\Telefon',
    scanProgress: [],
    scanResult: {
      ok: {
        scanId: SCAN_ID,
        photosFound: entries.length,
        unreadableEntries: 0,
        withoutExifDate: 0,
      },
    },
    timeline: { entries },
    series: 'byGap',
    ...extra,
  };
}

/** Waits until every timeline tile shows its thumbnail and every other image has loaded. */
async function imagesLoaded(page: Page): Promise<void> {
  await page.waitForFunction(() => {
    const tiles = Array.from(document.querySelectorAll('[data-testid="timeline-tile"]'));
    const images = Array.from(document.images);
    return (
      tiles.every((tile) => tile.querySelector('img') !== null) &&
      images.every((image) => image.complete && image.naturalWidth > 0)
    );
  });
}

test.skip(entries.length === 0, 'Run scripts/make-demo-gallery.py first to build demo/.');

for (const scheme of ['light', 'dark'] as const) {
  test.describe(`${scheme} mode`, () => {
    test.use({ colorScheme: scheme, viewport: SIZE, deviceScaleFactor: 1 });

    test('timeline', async ({ page }) => {
      await serveDemo(page);
      await mockTauri(page, demoMock());
      await page.goto('/');
      await page.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }).click();
      await expect(page.getByRole('listitem').first()).toBeVisible();
      await imagesLoaded(page);

      await page.screenshot({ path: `${IMAGES}timeline-${scheme}.jpg`, ...SHOT });
    });

    test('viewer', async ({ page }) => {
      await serveDemo(page);
      await mockTauri(
        page,
        demoMock({
          viewerRequest: { id: entry(DOG_SHARP).id, scanId: SCAN_ID, mode: 'series', gap: 10 },
          rejected: [entry(DOG_BLURRED).id],
        }),
      );
      await page.goto('/?window=viewer');
      await expect(page.getByRole('img', { name: DOG_SHARP })).toBeVisible();
      await imagesLoaded(page);

      await page.screenshot({ path: `${IMAGES}viewer-${scheme}.jpg`, ...SHOT });
    });
  });
}
