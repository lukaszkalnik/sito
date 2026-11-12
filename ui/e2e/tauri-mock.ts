import type { Page } from '@playwright/test';

type Progress =
  { phase: 'searching'; found: number } | { phase: 'reading'; done: number; total: number };

interface Summary {
  scanId: number;
  photosFound: number;
  unreadableEntries: number;
  withoutDate: number;
  refreshing?: boolean;
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
  series?:
    { start: number; len: number; label?: unknown }[] | { every: number; size: number } | 'byGap';
  /**
   * Display order returned by `get_series` (indices of the timeline entries).
   * Default: the timeline order.
   */
  order?: number[];
  /** Request returned by `current_viewer_request` in the viewer window. Default: none. */
  viewerRequest?: { id: number; scanId: number; mode: 'series' | 'all'; gap: number };
  /** Fields replacing those of the photo shown by `get_viewer_photo`. Default: none. */
  viewerEntry?: {
    name?: string;
    relativeDir?: string;
    timeSource?: 'exif' | 'file' | 'takeout' | 'takeoutCorrected';
  };
  /** Ids of photos already accepted. Default: none. */
  accepted?: number[];
  /** Ids of photos already rejected (and undoable), oldest first. Default: none. */
  rejected?: number[];
  /** Ids among `rejected` that went into the trash. Default: none. */
  trashed?: number[];
  /** Whether undo can restore from the trash (not on macOS). Default: `supported`. */
  trashRestore?: 'supported' | 'unsupported';
  /**
   * A background refresh after `start_scan` (when its summary says
   * `refreshing`): `scan-refreshed` with `summary` after `afterMs`.
   */
  refresh?: { summary: Summary; afterMs: number };
  /**
   * Reading EXIF in the background after `start_scan`: one `scan-completing`
   * event per step (for the scan's id), `everyMs` apart.
   */
  completing?: { steps: { done: number; total: number }[]; everyMs: number };
  /**
   * Newer version returned by `check_for_update` (the running one is 0.9.0).
   * Default: none. `install_update` streams `installProgress`, then never
   * returns, like the app that the installer closes.
   */
  update?: { version: string; notes: string | null; install: 'inApp' | 'manual' };
  /** Progress messages streamed by `install_update`. Default: none. */
  installProgress?: unknown[];
  /** Error thrown by `open_release_page`. Default: none, the page opens. */
  openReleasePageError?: { code: string };
}

/** Commands the page invoked, with their arguments, for assertions. */
export async function invokedCommands(page: Page): Promise<{ cmd: string; args: unknown }[]> {
  return page.evaluate(
    () => (window as unknown as { __invoked: { cmd: string; args: unknown }[] }).__invoked,
  );
}

/** Sends an event to the page, like a click on a native menu item or a message from the viewer. */
export async function emitTauriEvent(
  page: Page,
  event: string,
  payload: unknown = null,
): Promise<void> {
  await page.evaluate(
    ([name, data]) =>
      (
        window as unknown as { __emitTauriEvent: (event: string, payload: unknown) => void }
      ).__emitTauriEvent(name as string, data),
    [event, payload] as const,
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

    // Ids of rejected photos, newest last; "Cofnij" undoes them newest first.
    const rejected: number[] = [...(m.rejected ?? [])];
    // Ids of photos rejected into the trash.
    const trashed = new Set<number>(m.trashed ?? []);
    /** The newest rejection undo may offer, like the backend's `undoable_rejections`. */
    const undoableIndex = () =>
      m.trashRestore === 'unsupported'
        ? rejected.findLastIndex((id) => !trashed.has(id))
        : rejected.length - 1;
    /** The backend's `UndoStateDto`. */
    const undoState = () => {
      if (undoableIndex() >= 0) return { kind: 'available' };
      return { kind: rejected.length > 0 ? 'trashOnly' : 'none' };
    };
    // Ids of accepted photos.
    const accepted = new Set<number>(m.accepted ?? []);
    // Event name → callback ids registered with `listen`.
    const listeners = new Map<string, number[]>();
    const emit = (event: string, payload: unknown = null) =>
      (listeners.get(event) ?? []).forEach((handler) =>
        callbacks.get(handler)?.({ event, id: 0, payload }),
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
          timeSource: t.fileDateEvery !== undefined && i % t.fileDateEvery === 0 ? 'file' : 'exif',
          format: 'jpeg',
        };
      });
    }

    /** Mirrors `find_time_series` in core: EXIF-dated neighbours at most the gap apart. */
    function seriesByGap(gapSeconds: number): unknown[] {
      // Like the backend: series are found including rejected photos and keep
      // their shape while any of their photos is left.
      const entries = allPhotos() as { id: number; takenAt: string; timeSource: string }[];
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
      entries: { takenAt: string; timeSource: string }[],
      gapSeconds: number,
    ): { start: number; len: number }[] {
      const result = [];
      let start = 0;
      for (let i = 1; i <= entries.length; i++) {
        const newer = entries[i - 1];
        const older = entries[i];
        const together =
          older !== undefined &&
          newer.timeSource !== 'file' &&
          older.timeSource !== 'file' &&
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

    /** Mirrors `get_viewer_photo`: the photo with its series and the way on (ids). */
    function viewerPhoto(request: { id: number; mode: string; gap: number }): unknown {
      const entries = timeline() as { id: number }[];
      const ranges = series(request.gap) as { start: number; len: number }[];
      const position = entries.findIndex((e) => e.id === request.id);
      if (position < 0) return null;
      const order =
        request.mode === 'all'
          ? entries.map((_, i) => i)
          : ranges.flatMap((s) => Array.from({ length: s.len }, (_, i) => s.start + i));
      const previous = order.filter((p) => p < position).at(-1);
      const next = order.find((p) => p > position);
      const own = ranges.find((s) => position >= s.start && position < s.start + s.len);
      let after = next ?? previous;
      if (own && own.len > 1) {
        after = position + 1 < own.start + own.len ? position + 1 : position - 1;
      }
      const idAt = (p: number | undefined) => (p === undefined ? null : entries[p].id);
      return {
        entry: { ...entries[position], ...m.viewerEntry },
        previous: idAt(previous),
        next: idAt(next),
        series: own ? entries.slice(own.start, own.start + own.len) : [],
        afterReject: idAt(after),
        details: { sizeBytes: 7_835_038, resolution: { width: 5472, height: 3648 } },
      };
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
            const refresh = m.refresh;
            if (refresh) setTimeout(() => emit('scan-refreshed', refresh.summary), refresh.afterMs);
            const { scanId } = m.scanResult.ok;
            m.completing?.steps.forEach((step, i) =>
              setTimeout(
                () => emit('scan-completing', { scanId, ...step }),
                (i + 1) * (m.completing?.everyMs ?? 0),
              ),
            );
            return m.scanResult.ok;
          }
          case 'get_timeline':
            return timeline();
          case 'get_series': {
            const ranges = series((args.maxGapSeconds as number | undefined) ?? 10) as object[];
            return {
              order: m.order ?? timeline().map((_, i) => i),
              series: ranges.map((range) => ({ label: { kind: 'plain' }, ...range })),
            };
          }
          case 'get_viewer_photo':
            return viewerPhoto(args.request as { id: number; mode: string; gap: number });
          case 'open_viewer':
            return null;
          case 'current_viewer_request':
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
            if (args.destination === 'trash') trashed.add(id);
            emit('timeline-changed');
            return { name: nameOf(id) };
          }
          case 'undo_reject': {
            const index = undoableIndex();
            if (index < 0) return null;
            const [id] = rejected.splice(index, 1);
            trashed.delete(id);
            emit('timeline-changed');
            return { id, name: nameOf(id) };
          }
          case 'accept_photo':
          case 'unaccept_photo': {
            const id = args.id as number;
            const accept = cmd === 'accept_photo';
            if (accept !== accepted.has(id)) {
              if (accept) accepted.add(id);
              else accepted.delete(id);
            }
            emit('accepted-changed');
            return { name: nameOf(id) };
          }
          case 'get_accepted_ids':
            return [...accepted].filter((id) => !rejected.includes(id));
          case 'check_for_update':
            return { currentVersion: '0.9.0', update: m.update ?? null };
          case 'install_update': {
            const send = callbacks.get((args.onProgress as { id: number }).id);
            (m.installProgress ?? []).forEach((message, index) => send?.({ index, message }));
            return new Promise(() => {});
          }
          case 'open_release_page':
            if (m.openReleasePageError !== undefined) throw m.openReleasePageError;
            return null;
          case 'plugin:app|version':
            return '0.9.0';
          case 'get_undo_state':
            return undoState();
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
    // Used by `emitTauriEvent`.
    Object.defineProperty(window, '__emitTauriEvent', { value: emit });
    // Used by `unlisten` of the event API.
    Object.defineProperty(window, '__TAURI_EVENT_PLUGIN_INTERNALS__', {
      value: { unregisterListener() {} },
    });
  }, mock);
}
