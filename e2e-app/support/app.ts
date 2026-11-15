import { $, $$, browser, expect } from '@wdio/globals';

import { startAppAgain, waitForAppExit } from './driver.ts';
import { plural, t } from './strings.ts';
import { requestClose, type WindowLabel } from './windows.ts';

/** The `localStorage` key of the folder the app reopens on start (slice 9). */
const LAST_FOLDER_KEY = 'sito.lastFolder';

/**
 * Opens `dir` in the main window the way the app reopens its last folder on
 * start: the folder picker is native and out of WebDriver's reach.
 */
export async function openFolder(dir: string): Promise<void> {
  await switchToWindow('main');
  await browser.execute((key, path) => localStorage.setItem(key, path), LAST_FOLDER_KEY, dir);
  await browser.execute(() => location.reload());
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
  if (series !== undefined) await expect($$('h3')).toBeElementsArrayOfSize(series);
}

/** The tile of photo `name` in the main window's grid. */
export const tile = (name: string) => $(`button[aria-label="${t('timeline.open', { name })}"]`);

/** The photo shown in the viewer, by its name. */
export const shownPhoto = (name: string) => $(`img[alt="${name}"]`);

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

/** Waits until the viewer window shows photo `name`. */
export async function expectViewerShows(name: string): Promise<void> {
  await switchToWindow('viewer');
  // Polled by hand: WebKitWebDriver reports a stale element instead of none
  // while React swaps the loading photo for the shown one.
  await browser.waitUntil(
    () =>
      shownPhoto(name)
        .isDisplayed()
        .catch(() => false),
    {
      timeoutMsg: `the viewer does not show ${name}`,
    },
  );
}

/**
 * Closes the viewer with „Zamknij” and the main window like its close button,
 * and waits until the app has quit.
 */
export async function quitApp(): Promise<void> {
  if ((await windowCount()) > 1) {
    await switchToWindow('viewer');
    await button(t('viewer.close')).click();
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
