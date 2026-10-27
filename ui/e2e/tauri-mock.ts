import type { Page } from '@playwright/test';

type Progress =
  { phase: 'searching'; found: number } | { phase: 'reading'; done: number; total: number };

interface Summary {
  scanId: number;
  photosFound: number;
  unreadableEntries: number;
  withoutExifDate: number;
}

/** Canned backend answers for one test. */
export interface TauriMock {
  /** Folder returned by the folder dialog, or `null` for "cancelled". */
  pickedFolder: string | null;
  /** Progress messages sent before `start_scan` resolves. */
  scanProgress: Progress[];
  /** Result of `start_scan`, or an error object it rejects with. */
  scanResult: { ok: Summary } | { err: { code: string } };
  /**
   * Timeline returned by `get_timeline`: either explicit entries or a
   * generated one with `count` photos, one every `stepSeconds`, newest first
   * starting at `newest` (local ISO time).
   */
  timeline?:
    | { entries: unknown[] }
    | { count: number; newest: string; stepSeconds: number; fileDateEvery?: number };
  /**
   * Series returned by `get_series`: explicit ranges, or generated as a
   * series of `size` photos starting at every `every`-th photo. Default: none.
   */
  series?: { start: number; len: number }[] | { every: number; size: number };
}

/**
 * Installs a fake `window.__TAURI_INTERNALS__` so the UI runs in a plain
 * browser against canned command results instead of the Rust backend.
 */
export async function mockTauri(page: Page, mock: TauriMock): Promise<void> {
  await page.addInitScript((m: TauriMock) => {
    const callbacks = new Map<number, (payload: unknown) => void>();
    let nextId = 1;

    const pad = (n: number) => String(n).padStart(2, '0');
    const localIso = (d: Date) =>
      `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T` +
      `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;

    function timeline(): unknown[] {
      const t = m.timeline;
      if (!t) return [];
      if ('entries' in t) return t.entries;
      const newest = new Date(t.newest).getTime();
      return Array.from({ length: t.count }, (_, i) => ({
        id: i,
        name: `IMG_${String(t.count - i).padStart(5, '0')}.jpg`,
        relativeDir: 'Camera',
        relativePath: `Camera\\IMG_${String(t.count - i).padStart(5, '0')}.jpg`,
        takenAt: localIso(new Date(newest - i * t.stepSeconds * 1000)),
        fromFileDate: t.fileDateEvery !== undefined && i % t.fileDateEvery === 0,
      }));
    }

    function series(): unknown[] {
      const s = m.series;
      if (!s) return [];
      if (Array.isArray(s)) return s;
      const count = timeline().length;
      const result = [];
      for (let start = 0; start + s.size <= count; start += s.every) {
        result.push({ start, len: s.size });
      }
      return result;
    }

    const internals = {
      // Windows-style URLs, as Tauri builds them for custom protocols there.
      convertFileSrc(filePath: string, protocol: string) {
        return `http://${protocol}.localhost/${encodeURIComponent(filePath)}`;
      },
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
          case 'get_timeline':
            return timeline();
          case 'get_series':
            return series();
          default:
            throw new Error(`Unmocked Tauri command: ${cmd}`);
        }
      },
    };

    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: internals });
  }, mock);
}
