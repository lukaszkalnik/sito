import { expect, test, type Page } from '@playwright/test';

import { mockTauri, type TauriMock } from './tauri-mock';

// A long Polish path and a summary with every part, as on a real gallery.
const FOLDER = 'C:\\Users\\Łukasz\\Pictures\\Skany z albumów rodzinnych\\Wakacje nad morzem';
const SUMMARY = { scanId: 3, photosFound: 1461, unreadableEntries: 2, withoutDate: 17 };

const POLISH: (typeof LANGUAGES)[number] = {
  info: { language: 'pl', locale: 'pl-PL', choice: 'system' },
  chooseFolder: 'Wybierz folder ze zdjęciami',
};

/** The smallest window width (`tauri.conf.json`), a small and a large laptop screen. */
const WIDTHS = [900, 1280, 1440];

/** The language of each run and its „Wybierz folder ze zdjęciami”. */
const LANGUAGES: readonly { info: NonNullable<TauriMock['language']>; chooseFolder: string }[] = [
  {
    info: { language: 'en', locale: 'en-GB', choice: 'en' },
    chooseFolder: 'Choose a photo folder',
  },
  { info: { language: 'de', locale: 'de-DE', choice: 'de' }, chooseFolder: 'Fotoordner wählen' },
];

async function open(page: Page, width: number, language = POLISH) {
  // The trash destination is shown bold, the widest look of the bar.
  await page.addInitScript(() => localStorage.setItem('sito.rejectDestination', 'trash'));
  await mockTauri(page, {
    pickedFolder: FOLDER,
    scanProgress: [],
    scanResult: { ok: { ...SUMMARY, refreshing: true } },
    refresh: { summary: SUMMARY, afterMs: 1500 },
    timeline: { count: 1461, newest: '2024-06-01T12:00:00', stepSeconds: 600 },
    series: { every: 10, size: 3 },
    language: language.info,
  });
  await page.setViewportSize({ width, height: 800 });
  await page.goto('/');
  await page.getByRole('button', { name: language.chooseFolder }).click();
  await expect(page.getByTestId('timeline')).toBeVisible();
}

// A `header` inside `main` has no landmark role, so the bar is found by its class.
const bar = (page: Page) => page.locator('.timeline-bar');

/** Boxes of the bar's parts: title, folder button, path, summary and each control. */
async function partBoxes(page: Page) {
  return bar(page).evaluate((header) =>
    [
      ...header.querySelectorAll(
        ':scope > h1, .folder-info > *, .folder-info-text > *, .timeline-bar-controls > *',
      ),
    ]
      .filter((part) => !part.classList.contains('folder-info-text'))
      .map((part) => {
        const { left, right, top, bottom } = part.getBoundingClientRect();
        return {
          name: part.textContent || part.getAttribute('aria-label'),
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

      await bar(page).screenshot({ path: testInfo.outputPath(`bar-${width}.png`) });
    });

    test('keeps the last folder of the path and the counts readable', async ({ page }) => {
      // At the minimum width both lines are cut; their tooltips hold the full text.
      test.skip(width < 1280, 'too narrow for both lines');
      const cut = (text: string) =>
        page
          .getByText(text, { exact: true })
          .evaluate((element) => element.scrollWidth > element.clientWidth);

      expect(await cut('Wakacje nad morzem')).toBe(false);
      expect(await cut('1461 zdjęć · 146 serii')).toBe(false);
      await expect(page.getByTitle(FOLDER)).toBeVisible();
    });

    test('stays one row high', async ({ page }) => {
      expect((await bar(page).boundingBox())?.height).toBeLessThan(70);
    });

    test('keeps its height while the folder is checked for changes', async ({ page }) => {
      await expect(page.getByText('Sprawdzanie zmian…')).toBeVisible();
      const during = (await bar(page).boundingBox())?.height;

      await expect(page.getByText('Sprawdzanie zmian…')).toHaveCount(0);

      expect((await bar(page).boundingBox())?.height).toBe(during);
    });
  });
}

// English and German labels are longer than the Polish ones.
for (const language of LANGUAGES) {
  test(`keeps the bar one row with its parts apart at 900 px in ${language.info.language}`, async ({
    page,
  }, testInfo) => {
    await open(page, 900, language);
    const boxes = await partBoxes(page);
    const overlapping = boxes.flatMap((a, i) =>
      boxes.slice(i + 1).flatMap((b) => (overlap(a, b) ? [[a.name, b.name]] : [])),
    );
    expect(overlapping).toEqual([]);
    const scroll = await bar(page).evaluate((header) => [header.scrollWidth, header.clientWidth]);
    expect(scroll[0]).toBeLessThanOrEqual(scroll[1]);
    expect((await bar(page).boundingBox())?.height).toBeLessThan(70);
    await bar(page).screenshot({
      path: testInfo.outputPath(`bar-900-${language.info.language}.png`),
    });
  });
}
