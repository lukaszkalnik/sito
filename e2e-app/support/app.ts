import { $, $$, browser } from '@wdio/globals';

import { startAppAgain, waitForAppExit } from './driver.ts';
import { plural, t } from './strings.ts';
import { requestClose, type WindowLabel } from './windows.ts';

/** The `localStorage` key of the folder the app reopens on start (slice 9). */
const LAST_FOLDER_KEY = 'sito.lastFolder';

/**
 * Opens `dir` in the main window the way the app reopens its last folder on
 * start: the folder picker is native and out of WebDriver's reach. Waits for
 * the start screen first, so the page the reload replaces has started and
 * opens nothing itself: each spec opens its folder exactly once.
 */
export async function openFolder(dir: string): Promise<void> {
  await switchToWindow('main');
  await waitForStartScreen();
  await browser.execute((key, path) => localStorage.setItem(key, path), LAST_FOLDER_KEY, dir);
  // WebDriver's own reload: a script that navigates away never returns on WebView2.
  await browser.refresh();
}

/** Waits until the main window shows the start screen, ready to pick a folder. */
async function waitForStartScreen(): Promise<void> {
  const choose = button(t('scan.chooseFolder'));
  await browser.waitUntil(() => choose.isEnabled().catch(() => false), {
    timeoutMsg: 'the main window never showed its start screen',
  });
}

/** The full summary of the main window's status line („Znaleziono 6 zdjęć. …”). */
async function summary(): Promise<string> {
  const status = $('[role="status"][title]');
  return (await status.isExisting()) ? ((await status.getAttribute('title')) ?? '') : '';
}

/** Waits until the main window shows `photos` photos (in `series` series, if given). */
export async function expectCounts(photos: number, series?: number): Promise<void> {
  const found = t('scan.result', { photos: plural('scan.photoCount', photos) });
  await browser.waitUntil(async () => (await summary()).startsWith(found), {
    timeoutMsg: `the main window never said „${found}”`,
  });
  if (series === undefined) return;
  // Counted by hand: WebKitWebDriver reports a stale element instead of a
  // result while the grid re-renders. Series headers are the grid's h3.
  await browser.waitUntil(
    async () =>
      (await $$('h3')
        .length.then(Number)
        .catch(() => -1)) === series,
    { timeoutMsg: `the main window does not show ${series} series` },
  );
}

/** CSS selector of the tile of photo `name` in the main window's grid. */
const tileSelector = (name: string) => `button[aria-label="${t('timeline.open', { name })}"]`;

/** The tile of photo `name` in the main window's grid. */
export const tile = (name: string) => $(tileSelector(name));

/** XPath of the buttons named `name`: by their label, or their text. */
export const buttonSelector = (name: string) =>
  `//button[@aria-label="${name}" or normalize-space()="${name}"]`;

/** The one button named `name`. */
export const button = (name: string) => $(buttonSelector(name));

/** Switches WebDriver to window `label`, waiting for it to exist. */
export async function switchToWindow(label: WindowLabel): Promise<void> {
  await browser.waitUntil(async () => findWindow(label), { timeoutMsg: `no ${label} window` });
}

/** Switches to window `label` if it is open; whether it is. */
async function findWindow(label: WindowLabel): Promise<boolean> {
  for (const handle of await browser.getWindowHandles()) {
    await browser.switchToWindow(handle);
    const isViewer = (await browser.getUrl()).includes('window=viewer');
    if (isViewer === (label === 'viewer')) return true;
  }
  return false;
}

/** The number of the app's open windows. */
export async function windowCount(): Promise<number> {
  return (await browser.getWindowHandles()).length;
}

/**
 * Waits until the main window's grid shows the tile of photo `name`: it is in
 * the document and has a size. Checked in the page by selector, as
 * WebKitWebDriver sometimes cannot hand out a found element („Stale element
 * found when trying to create the node handle”) for many seconds.
 */
export async function waitForTile(name: string): Promise<void> {
  const shown = () =>
    browser.execute((selector) => {
      const button = document.querySelector(selector);
      if (!button?.isConnected) return false;
      const { width, height } = button.getBoundingClientRect();
      const visible = 'checkVisibility' in button ? button.checkVisibility() : true;
      return width > 0 && height > 0 && visible;
    }, tileSelector(name));
  await browser.waitUntil(() => shown().catch(() => false), {
    timeoutMsg: `the tile of ${name} is not shown`,
  });
}

/**
 * Waits until the viewer window shows photo `name`: an image of that name has
 * loaded. Checked in the page, as WebKitWebDriver's own visibility check
 * sometimes misses a shown photo.
 */
export async function expectViewerShows(name: string): Promise<void> {
  await switchToWindow('viewer');
  const loaded = () =>
    browser.execute(
      (alt) =>
        [...document.images].some(
          (image) => image.alt === alt && image.complete && image.naturalWidth > 0,
        ),
      name,
    );
  await browser.waitUntil(() => loaded().catch(() => false), {
    timeoutMsg: `the viewer does not show ${name}`,
  });
}

/**
 * Closes the viewer with „Zamknij” and the main window like its close button,
 * and waits until the app has quit.
 */
export async function quitApp(): Promise<void> {
  if ((await windowCount()) > 1) {
    await switchToWindow('viewer');
    // The window may be gone before WebDriver gets the click's reply, which is
    // then lost; the closing is checked below.
    await button(t('viewer.close'))
      .click()
      .catch(() => {});
    await browser.waitUntil(async () => (await windowCount()) === 1, {
      timeoutMsg: 'the viewer did not close',
    });
  }
  await switchToWindow('main');
  await requestClose('main');
  await waitForAppExit();
}

/** Starts the app again after `quitApp`, with the same state, in a new session. */
export async function startAgain(): Promise<void> {
  await startAppAgain();
  await browser.reloadSession();
}
