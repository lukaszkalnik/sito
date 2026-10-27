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
    | {
        count: number;
        newest: string;
        stepSeconds: number;
        /** Repeating gaps in seconds between neighbours; overrides `stepSeconds`. */
        stepPattern?: number[];
        fileDateEvery?: number;
      };
  /**
   * Series returned by `get_series`: explicit ranges, generated as a series
   * of `size` photos starting at every `every`-th photo, or `'byGap'` to group
   * the timeline by the requested gap like the backend does. Default: none.
   */
  series?: { start: number; len: number }[] | { every: number; size: number } | 'byGap';
  /** Request returned by `take_viewer_request` in the viewer window. Default: none. */
  viewerRequest?: { id: number; scanId: number; mode: 'series' | 'all'; gap: number };
  /** Ids of photos already rejected (and undoable), oldest first. Default: none. */
  rejected?: number[];
}

/** Commands the page invoked, with their arguments, for assertions. */
export async function invokedCommands(page: Page): Promise<{ cmd: string; args: unknown }[]> {
  return page.evaluate(
    () => (window as unknown as { __invoked: { cmd: string; args: unknown }[] }).__invoked,
  );
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

    // Ids of rejected photos, newest last; undo pops them.
    const rejected: number[] = [...(m.rejected ?? [])];
    // Event name → callback ids registered with `listen`.
    const listeners = new Map<string, number[]>();
    const emit = (event: string) =>
      (listeners.get(event) ?? []).forEach((handler) =>
        callbacks.get(handler)?.({ event, id: 0, payload: null }),
      );

    /** The timeline as the backend returns it: without rejected photos. */
    function timeline(): unknown[] {
      return allPhotos().filter((photo) => !rejected.includes((photo as { id: number }).id));
    }

    function nameOf(id: number): string {
      const photo = allPhotos().find((p) => (p as { id: number }).id === id) as { name: string };
      return photo.name;
    }

    function allPhotos(): unknown[] {
      const t = m.timeline;
      if (!t) return [];
      if ('entries' in t) return t.entries;
      const newest = new Date(t.newest).getTime();
      const pattern = t.stepPattern ?? [t.stepSeconds];
      let offset = 0;
      return Array.from({ length: t.count }, (_, i) => {
        if (i > 0) offset += pattern[(i - 1) % pattern.length] * 1000;
        return {
          id: i,
          name: `IMG_${String(t.count - i).padStart(5, '0')}.jpg`,
          relativeDir: 'Camera',
          relativePath: `Camera\\IMG_${String(t.count - i).padStart(5, '0')}.jpg`,
          takenAt: localIso(new Date(newest - offset)),
          fromFileDate: t.fileDateEvery !== undefined && i % t.fileDateEvery === 0,
        };
      });
    }

    /** Mirrors `find_time_series` in core: EXIF-dated neighbours at most the gap apart. */
    function seriesByGap(gapSeconds: number): unknown[] {
      // Like the backend: series are found including rejected photos and keep
      // their shape while any of their photos is left.
      const entries = allPhotos() as { id: number; takenAt: string; fromFileDate: boolean }[];
      const keptBefore = [0];
      entries.forEach((e, i) =>
        keptBefore.push((keptBefore[i] ?? 0) + (rejected.includes(e.id) ? 0 : 1)),
      );
      return findSeries(entries, gapSeconds)
        .map(({ start, len }) => ({
          start: keptBefore[start] ?? 0,
          len: (keptBefore[start + len] ?? 0) - (keptBefore[start] ?? 0),
        }))
        .filter((s) => s.len > 0);
    }

    function findSeries(
      entries: { takenAt: string; fromFileDate: boolean }[],
      gapSeconds: number,
    ): { start: number; len: number }[] {
      const result = [];
      let start = 0;
      for (let i = 1; i <= entries.length; i++) {
        const newer = entries[i - 1];
        const older = entries[i];
        const together =
          older !== undefined &&
          !newer.fromFileDate &&
          !older.fromFileDate &&
          new Date(newer.takenAt).getTime() - new Date(older.takenAt).getTime() <=
            gapSeconds * 1000;
        if (!together) {
          if (i - start >= 2) result.push({ start, len: i - start });
          start = i;
        }
      }
      return result;
    }

    function series(gapSeconds: number): unknown[] {
      const s = m.series;
      if (!s) return [];
      if (s === 'byGap') return seriesByGap(gapSeconds);
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
            return series((args.maxGapSeconds as number | undefined) ?? 10);
          case 'open_viewer':
            return null;
          case 'take_viewer_request':
            return m.viewerRequest ?? null;
          // Events and window calls have no effect in a plain browser.
          case 'plugin:event|listen': {
            const event = args.event as string;
            listeners.set(event, [...(listeners.get(event) ?? []), args.handler as number]);
            return nextId++;
          }
          case 'reject_photo': {
            const id = args.id as number;
            rejected.push(id);
            emit('timeline-changed');
            return { name: nameOf(id) };
          }
          case 'undo_reject': {
            const id = rejected.pop();
            if (id === undefined) return null;
            emit('timeline-changed');
            return { id, name: nameOf(id) };
          }
          case 'get_undo_state':
            return { canUndo: rejected.length > 0 };
          case 'plugin:event|unlisten':
          case 'plugin:event|emit_to':
          case 'plugin:window|set_title':
          case 'plugin:window|close':
            return null;
          default:
            throw new Error(`Unmocked Tauri command: ${cmd}`);
        }
      },
      metadata: {
        currentWindow: { label: new URLSearchParams(location.search).get('window') ?? 'main' },
        currentWebview: { label: 'main', windowLabel: 'main' },
      },
    };

    // Records every command for `invokedCommands`.
    const invoked: { cmd: string; args: unknown }[] = [];
    Object.defineProperty(window, '__invoked', { value: invoked });
    const invoke = internals.invoke;
    internals.invoke = (cmd, args) => {
      invoked.push({ cmd, args });
      return invoke(cmd, args);
    };

    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: internals });
    // Used by `unlisten` of the event API.
    Object.defineProperty(window, '__TAURI_EVENT_PLUGIN_INTERNALS__', {
      value: { unregisterListener() {} },
    });
  }, mock);
}
