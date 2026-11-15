import { browser, expect } from '@wdio/globals';

import { expectCounts, openFolder, tile } from '../support/app.ts';
import { EXPECTED, SERIES_ORDER, makeGallery } from '../support/gallery.ts';

describe('scan', () => {
  it('finds the photos and series of a folder and loads their thumbnails', async () => {
    const gallery = makeGallery();

    await openFolder(gallery.dir);

    await expectCounts(EXPECTED.photos, EXPECTED.series);
    for (const name of SERIES_ORDER) await expect(tile(name)).toBeDisplayed();
    // A thumbnail with a size came through the app's thumb: protocol.
    await browser.waitUntil(
      async () =>
        browser.execute(() => {
          const images = [...document.querySelectorAll('img')] as HTMLImageElement[];
          return images.length > 0 && images.every((image) => image.naturalWidth > 0);
        }),
      { timeoutMsg: 'the thumbnails did not load' },
    );
  });
});
