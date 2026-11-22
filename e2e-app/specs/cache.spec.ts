import { existsSync, mkdtempSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { $, browser, expect } from '@wdio/globals';

import {
  expectCounts,
  openFolder,
  quitApp,
  startAgain,
  switchToWindow,
  waitForTile,
} from '../support/app.ts';
import { IDENTIFIER } from '../support/appState.ts';
import { EXPECTED, SERIES_ORDER, makeGallery } from '../support/gallery.ts';
import { t } from '../support/strings.ts';

/** The cache folder the app uses unless another is chosen (`app_cache_dir`). */
const DEFAULT_DIR = join(process.env.LOCALAPPDATA ?? '', IDENTIFIER);

/** What `move_cache` answers: the settings, or the error code it failed with. */
type MoveAnswer = { folder: string } | { code: string };

/**
 * Moves the cache into `folder` (`null`: back to the default) through the
 * app's command, as „Zmień…” does after the folder picker, which is native
 * and out of WebDriver's reach. Tries again while a folder is being read.
 */
async function moveCache(folder: string | null): Promise<string> {
  // Kept in an object: TypeScript does not follow assignments in the callback.
  const last: { answer: MoveAnswer } = { answer: { code: 'busy' } };
  await browser.waitUntil(
    async () => {
      last.answer = await browser.execute(async (target) => {
        const tauri = (
          window as unknown as {
            __TAURI_INTERNALS__: {
              invoke: (cmd: string, args: unknown) => Promise<unknown>;
              transformCallback: (callback: (message: unknown) => void) => number;
            };
          }
        ).__TAURI_INTERNALS__;
        // A channel as `Channel` sends it: progress is not checked here.
        const channel = `__CHANNEL__:${tauri.transformCallback(() => {})}`;
        try {
          return (await tauri.invoke('move_cache', {
            folder: target,
            onProgress: channel,
          })) as MoveAnswer;
        } catch (err) {
          return err as MoveAnswer;
        }
      }, folder);
      return !('code' in last.answer) || last.answer.code !== 'busy';
    },
    { timeout: 30_000, timeoutMsg: 'the cache stayed busy' },
  );
  const { answer } = last;
  if ('code' in answer) throw new Error(`move_cache failed: ${answer.code}`);
  return answer.folder;
}

/** Shows the cache settings as the menu item does, and waits for `folder` in them. */
async function expectDialogShows(folder: string): Promise<void> {
  await browser.execute(async () => {
    const tauri = (
      window as unknown as {
        __TAURI_INTERNALS__: { invoke: (cmd: string, args: unknown) => Promise<unknown> };
      }
    ).__TAURI_INTERNALS__;
    await tauri.invoke('plugin:event|emit', { event: 'settings-show-cache', payload: null });
  });
  const dialog = $(`//dialog[.//h2[normalize-space()="${t('cache.title')}"]]`);
  await dialog.waitForDisplayed();
  await $(`//dialog//*[@title="${folder}"]`).waitForExist({
    timeoutMsg: `the cache settings do not show ${folder}`,
  });
  await browser.keys('Escape');
}

/** Whether `dir` holds a cache: the photo cache and at least one thumbnail. */
function holdsCache(dir: string): boolean {
  const thumbnails = join(dir, 'thumbnails');
  return (
    existsSync(join(dir, 'photos.db')) &&
    existsSync(thumbnails) &&
    readdirSync(thumbnails, { recursive: true }).some((name) => String(name).endsWith('.jpg'))
  );
}

/** Whether `dir` holds anything of a cache. */
function holdsAnyCache(dir: string): boolean {
  return existsSync(join(dir, 'photos.db')) || existsSync(join(dir, 'thumbnails'));
}

// Choosing the cache folder is offered on Windows only (slice 15).
(process.platform === 'win32' ? describe : describe.skip)('cache folder', () => {
  it('moves the cache with an open gallery, keeps it across a restart and moves it back', async () => {
    const gallery = makeGallery();
    await openFolder(gallery.dir);
    await expectCounts(EXPECTED.photos, EXPECTED.series);
    for (const name of SERIES_ORDER) await waitForTile(name);
    expect(holdsCache(DEFAULT_DIR)).toBe(true);

    const chosen = mkdtempSync(join(tmpdir(), 'sito-e2e-cache-'));
    const target = join(chosen, 'Sito-cache');
    expect(await moveCache(chosen)).toBe(target);

    expect(holdsCache(target)).toBe(true);
    expect(existsSync(join(target, '.nomedia'))).toBe(true);
    expect(holdsAnyCache(DEFAULT_DIR)).toBe(false);
    await expectDialogShows(target);

    // The next start uses the chosen folder: the gallery opens from its cache
    // and nothing is created in the default folder.
    await quitApp();
    await startAgain();
    await switchToWindow('main');
    await expectCounts(EXPECTED.photos, EXPECTED.series);
    for (const name of SERIES_ORDER) await waitForTile(name);
    expect(holdsCache(target)).toBe(true);
    expect(holdsAnyCache(DEFAULT_DIR)).toBe(false);

    expect(await moveCache(null)).toBe(DEFAULT_DIR);
    expect(holdsCache(DEFAULT_DIR)).toBe(true);
    expect(existsSync(target)).toBe(false);
    await expectDialogShows(DEFAULT_DIR);
  });
});
