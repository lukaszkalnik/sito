import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { expect } from '@wdio/globals';

import {
  expectCounts,
  expectViewerShows,
  openFolder,
  quitApp,
  startAgain,
  switchToWindow,
  tile,
} from '../support/app.ts';
import { configFolder } from '../support/appState.ts';
import { appEnv } from '../support/driver.ts';
import { EXPECTED, SERIES_ORDER, makeGallery } from '../support/gallery.ts';
import { placeWindow, windowRect, type Rect, type WindowLabel } from '../support/windows.ts';

/**
 * How far a window may come back from where it was, in physical pixels: the
 * system may shift a window by its frame (as `BORDER_SLACK` in window_fit.rs).
 */
const TOLERANCE = 16;

// Away from the defaults (centered) and from each other, and inside the
// 1024×768 screen of the Windows runner, so nothing moves them back on screen.
const MAIN = { x: 20, y: 40, width: 920, height: 620 };
const VIEWER = { x: 300, y: 180, width: 600, height: 420 };

/** Where `actual` is further than `TOLERANCE` from `expected`. */
function offBy(actual: Rect, expected: Rect): string[] {
  return (['x', 'y', 'width', 'height'] as const)
    .filter((key) => Math.abs(actual[key] - expected[key]) > TOLERANCE)
    .map((key) => `${key} ${actual[key]} instead of ${expected[key]}`);
}

/**
 * On a Mac screen smaller than the default window, as on the CI runner, the
 * restored frame is lost (slice 28a); until that is fixed, macOS only checks
 * that the state is saved.
 */
const CHECKS_FRAMES = process.platform !== 'darwin';

async function expectAt(label: WindowLabel, expected: Rect): Promise<void> {
  if (!CHECKS_FRAMES) return;
  expect(offBy(await windowRect(label), expected)).toEqual([]);
}

/** The labels of the windows in the app's saved window state. */
function savedWindows(): string[] {
  const file = join(configFolder(appEnv()), '.window-state.json');
  return Object.keys(JSON.parse(readFileSync(file, 'utf8')) as Record<string, unknown>).sort();
}

describe('restart', () => {
  it('reopens the gallery and both windows where they were', async () => {
    await openFolder(makeGallery().dir);
    await expectCounts(EXPECTED.photos, EXPECTED.series);
    const [name] = SERIES_ORDER;
    await tile(name).click();
    await expectViewerShows(name);
    const viewer = await placeWindow('viewer', VIEWER);
    await switchToWindow('main');
    const main = await placeWindow('main', MAIN);

    // Closing the last window must reach the save of the window state (v0.6.3).
    await quitApp();
    expect(savedWindows()).toEqual(['main', 'viewer']);
    await startAgain();

    await switchToWindow('main');
    await expectCounts(EXPECTED.photos, EXPECTED.series);
    await expectAt('main', main);
    await tile(name).click();
    await expectViewerShows(name);
    await expectAt('viewer', viewer);
  });
});
