import { expect, test, type Page } from '@playwright/test';

import { mockTauri, type TauriMock } from './tauri-mock';

// 1×1 grey PNG, served for photos and thumbnails.
const PIXEL = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNoAAAAggCBd81ytgAAAABJRU5ErkJggg==',
  'base64',
);

/** The widest info: a long Takeout name, the longest date source note and a deep folder. */
const TAKEOUT: TauriMock['viewerEntry'] = {
  name: 'PXL_20230715_143012345.MP.jpg',
  relativeDir: 'Takeout/Zdjęcia Google/Zdjęcia z 2023',
  timeSource: 'takeoutCorrected',
};

/** Where „Zaakceptuj” and „Odrzuć” lose their labels (`ViewerBar.css`). */
/** A plain camera photo, as the mock shows it. */
const PLAIN: TauriMock['viewerEntry'] = {};

const BREAKPOINT = 780;

/** The viewer's minimum width (`src-tauri/src/viewer.rs`), the breakpoint, a small and a large screen. */
const WIDTHS = [480, BREAKPOINT, BREAKPOINT + 1, 900, 1440];

async function open(
  page: Page,
  width: number,
  viewerEntry: TauriMock['viewerEntry'] = TAKEOUT,
  accepted: number[] = [],
) {
  for (const protocol of ['photo', 'thumb']) {
    await page.route(`http://${protocol}.localhost/**`, (route) =>
      route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL }),
    );
  }
  await mockTauri(page, {
    pickedFolder: null,
    scanProgress: [],
    scanResult: { err: { code: 'internal' } },
    timeline: { count: 30, newest: '2024-06-01T12:00:00', stepSeconds: 1200 },
    series: { every: 10, size: 3 },
    viewerRequest: { id: 1, scanId: 5, mode: 'series', gap: 10 },
    viewerEntry,
    accepted,
  });
  await page.setViewportSize({ width, height: 600 });
  await page.goto('/?window=viewer');
  await expect(bar(page)).toBeVisible();
}

// A `header` inside `main` has no landmark role, so the bar is found by its class.
const bar = (page: Page) => page.locator('.viewer-bar');

/** Boxes of the bar's parts: the info and each button. */
async function partBoxes(page: Page) {
  return bar(page).evaluate((header) =>
    [...header.children].map((part) => {
      const { left, right, top, bottom } = part.getBoundingClientRect();
      return {
        name: part.getAttribute('aria-label') ?? part.className,
        left,
        right,
        top,
        bottom,
      };
    }),
  );
}

type Box = Awaited<ReturnType<typeof partBoxes>>[number];

const overlap = (a: Box, b: Box) =>
  a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;

/** Whether the element with exactly `text` is cut off. */
const isCut = (page: Page, text: string) =>
  page
    .locator('.viewer-bar')
    .getByText(text, { exact: true })
    .evaluate((element) => element.scrollWidth > element.clientWidth || element.clientWidth === 0);

for (const width of WIDTHS) {
  test.describe(`at ${width} px`, () => {
    test.beforeEach(async ({ page }) => open(page, width));

    test('keeps the parts of the bar apart and inside it', async ({ page }, testInfo) => {
      const boxes = await partBoxes(page);
      const overlapping = boxes.flatMap((a, i) =>
        boxes.slice(i + 1).flatMap((b) => (overlap(a, b) ? [[a.name, b.name]] : [])),
      );
      expect(overlapping).toEqual([]);

      const scroll = await bar(page).evaluate((header) => [header.scrollWidth, header.clientWidth]);
      expect(scroll[0]).toBeLessThanOrEqual(scroll[1]);
      expect(Math.max(...boxes.map((box) => box.right))).toBeLessThanOrEqual(width);

      await bar(page).screenshot({ path: testInfo.outputPath(`viewer-bar-${width}.png`) });
    });

    test('shows „Zaakceptuj” and „Odrzuć” as icons only below the breakpoint', async ({ page }) => {
      const narrow = width <= BREAKPOINT;
      for (const name of ['Zaakceptuj', 'Odrzuć']) {
        const button = page.getByRole('button', { name, exact: true });
        const box = await button.boundingBox();
        if (narrow) expect(box?.width).toBeLessThanOrEqual(30);
        else expect(box?.width).toBeGreaterThan(60);
      }
    });

    test('keeps the name and, on a large screen, the last folder readable', async ({ page }) => {
      test.skip(width < 900, 'too narrow for the whole name');
      expect(await isCut(page, 'PXL_20230715_143012345.MP.jpg')).toBe(false);
      if (width >= 1440) expect(await isCut(page, 'Zdjęcia z 2023')).toBe(false);
      await expect(page.getByTitle('Folder: Takeout/Zdjęcia Google/Zdjęcia z 2023')).toBeAttached();
    });
  });
}

for (const width of WIDTHS.filter((w) => w > BREAKPOINT)) {
  test(`keeps at least 300 px for the info at ${width} px`, async ({ page }) => {
    // Accepted, „Zaakceptuj” reads „Cofnij akceptację”, its widest label.
    await open(page, width, TAKEOUT, [1]);
    await expect(page.getByRole('button', { name: 'Cofnij akceptację' })).toBeVisible();
    const info = await page.locator('.viewer-info').boundingBox();
    expect(info?.width).toBeGreaterThanOrEqual(300);
  });
}

test('keeps the bar one height at every width and for every photo', async ({ browser }) => {
  const cases = WIDTHS.flatMap((width) => [TAKEOUT, PLAIN].map((entry) => ({ width, entry })));
  const heights = new Set<number>();
  for (const { width, entry } of cases) {
    const page = await browser.newPage();
    await open(page, width, entry);
    const name = entry?.name ?? 'IMG_00029.jpg';
    await expect(bar(page).getByText(name, { exact: true })).toBeVisible();
    heights.add((await bar(page).boundingBox())?.height ?? 0);
    await page.close();
  }
  expect([...heights]).toHaveLength(1);
});
