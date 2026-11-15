import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { browser } from '@wdio/globals';

import { windowRect, type WindowLabel } from './windows.ts';

/** What the viewer's backend holds: its last request, and the page of that photo. */
async function viewerBackend(): Promise<unknown> {
  return browser.execute(async () => {
    const tauri = (
      window as unknown as {
        __TAURI_INTERNALS__: { invoke: (cmd: string, args?: unknown) => Promise<unknown> };
      }
    ).__TAURI_INTERNALS__;
    const request = await tauri.invoke('current_viewer_request');
    const page = request === null ? null : await tauri.invoke('get_viewer_photo', { request });
    return { request, page };
  });
}

/**
 * Keeps what a failed test leaves for a look: a screenshot, the HTML and the
 * frame of each open window, and for the viewer what its backend holds.
 */
export async function keepWindows(into: string): Promise<void> {
  mkdirSync(into, { recursive: true });
  for (const [index, handle] of (await browser.getWindowHandles()).entries()) {
    await browser.switchToWindow(handle);
    const label: WindowLabel = (await browser.getUrl()).includes('window=viewer')
      ? 'viewer'
      : 'main';
    const file = (extension: string) => join(into, `${index}-${label}.${extension}`);
    await browser.saveScreenshot(file('png'));
    writeFileSync(file('html'), await browser.getPageSource());
    const details = {
      rect: await windowRect(label).catch(String),
      backend: label === 'viewer' ? await viewerBackend().catch(String) : undefined,
    };
    writeFileSync(file('json'), JSON.stringify(details, null, 2));
  }
}
