import type { Page } from '@playwright/test';

/** Canned backend answers for one test. */
export interface TauriMock {
  /** Folder returned by the folder dialog, or `null` for "cancelled". */
  pickedFolder: string | null;
  /** Progress messages sent before `start_scan` resolves. */
  scanProgress: { entriesSeen: number; photosFound: number }[];
  /** Result of `start_scan`, or an error object it rejects with. */
  scanResult:
    | { ok: { photosFound: number; entriesSeen: number; unreadableEntries: number } }
    | { err: { code: string } };
}

/**
 * Installs a fake `window.__TAURI_INTERNALS__` so the UI runs in a plain
 * browser against canned command results instead of the Rust backend.
 */
export async function mockTauri(page: Page, mock: TauriMock): Promise<void> {
  await page.addInitScript((m: TauriMock) => {
    const callbacks = new Map<number, (payload: unknown) => void>();
    let nextId = 1;

    const internals = {
      transformCallback(callback: (payload: unknown) => void) {
        const id = nextId++;
        callbacks.set(id, callback);
        return id;
      },
      unregisterCallback(id: number) {
        callbacks.delete(id);
      },
      async invoke(cmd: string, args: Record<string, unknown>) {
        switch (cmd) {
          case 'plugin:dialog|open':
            return m.pickedFolder;
          case 'start_scan': {
            // Channel messages are delivered as { index, message } to the channel callback.
            const channel = args.onProgress as { id: number };
            const send = callbacks.get(channel.id);
            m.scanProgress.forEach((message, index) => send?.({ index, message }));
            await new Promise((resolve) => setTimeout(resolve, 300));
            if ('err' in m.scanResult) {
              throw m.scanResult.err;
            }
            send?.({ index: m.scanProgress.length, end: true });
            return m.scanResult.ok;
          }
          default:
            throw new Error(`Unmocked Tauri command: ${cmd}`);
        }
      },
    };

    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: internals });
  }, mock);
}
