import { existsSync } from 'node:fs';

import { $$, browser } from '@wdio/globals';

import {
  button,
  buttonSelector,
  expectCounts,
  expectViewerShows,
  openFolder,
  switchToWindow,
  tile,
} from '../support/app.ts';
import { EXPECTED, SERIES_ORDER, makeGallery } from '../support/gallery.ts';
import { t } from '../support/strings.ts';

const [shown, next] = SERIES_ORDER;

describe('reject', () => {
  it('moves the shown photo into „Odrzucone” and back with „Cofnij”', async () => {
    const gallery = makeGallery();
    await openFolder(gallery.dir);
    await expectCounts(EXPECTED.photos, EXPECTED.series);
    await tile(shown).click();
    await expectViewerShows(shown);

    await button(t('reject.button')).click();

    await expectViewerShows(next);
    await browser.waitUntil(
      () => existsSync(gallery.rejectedPath(shown)) && !existsSync(gallery.path(shown)),
      { timeoutMsg: `${shown} was not moved into Odrzucone` },
    );
    await switchToWindow('main');
    await expectCounts(EXPECTED.photos - 1);

    await switchToWindow('viewer');
    // The bar's „Cofnij”; the toast has one too.
    await $$(buttonSelector(t('undo.button')))[0]?.click();

    await browser.waitUntil(
      () => existsSync(gallery.path(shown)) && !existsSync(gallery.rejectedPath(shown)),
      { timeoutMsg: `${shown} did not come back from Odrzucone` },
    );
    await switchToWindow('main');
    await expectCounts(EXPECTED.photos, EXPECTED.series);
  });
});
