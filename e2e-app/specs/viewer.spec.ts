import { $, browser, expect } from '@wdio/globals';

import {
  expectCounts,
  expectViewerShows,
  openFolder,
  switchToWindow,
  tile,
  windowCount,
} from '../support/app.ts';
import { EXPECTED, SERIES_ORDER, makeGallery } from '../support/gallery.ts';
import { t } from '../support/strings.ts';

const [canonNewer, canonOlder, pixelNewer] = SERIES_ORDER;

/** The photo the viewer's series strip marks as the shown one. */
async function currentInStrip(): Promise<string | null> {
  const strip = $(`nav[aria-label="${t('viewer.series')}"]`);
  return strip.$('button[aria-current="true"]').getAttribute('aria-label');
}

describe('viewer', () => {
  before(async () => {
    await openFolder(makeGallery().dir);
    await expectCounts(EXPECTED.photos, EXPECTED.series);
  });

  it('opens in a window of its own and shows the next photo in the same one', async () => {
    await tile(pixelNewer).click();

    // A freeze while the viewer opens (v0.6.2) fails here.
    await browser.waitUntil(async () => (await windowCount()) === 2, {
      timeoutMsg: 'the viewer window did not open',
    });
    await expectViewerShows(pixelNewer);

    await switchToWindow('main');
    await tile(canonOlder).click();

    await expectViewerShows(canonOlder);
    expect(await windowCount()).toBe(2);
  });

  it('steps through a series in the order of the grid and on to the next series', async () => {
    await switchToWindow('main');
    await tile(canonNewer).click();
    await expectViewerShows(canonNewer);
    await browser.waitUntil(async () => (await currentInStrip()) === canonNewer);

    await browser.keys('ArrowRight');
    await expectViewerShows(canonOlder);
    await browser.waitUntil(async () => (await currentInStrip()) === canonOlder, {
      timeoutMsg: `the strip does not mark ${canonOlder}`,
    });

    await browser.keys('ArrowRight');
    await expectViewerShows(pixelNewer);

    await browser.keys('ArrowLeft');
    await expectViewerShows(canonOlder);
  });
});
