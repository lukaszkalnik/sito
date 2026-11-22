import type { Page } from '@playwright/test';

/** The `CacheSettingsDto` the cache mock returns. */
interface CacheSettingsMock {
  folder: string;
  location: { kind: 'default' } | { kind: 'chosen' } | { kind: 'fallback'; chosen: string };
  platform: 'windows' | 'macos' | 'other';
  thumbnailLimitMb: number;
}

/** What the cache settings' commands answer (see `TauriMock.cache`). */
export interface CacheMock {
  settings?: CacheSettingsMock;
  size?: { thumbnails: number; photoData: number };
  moveProgress?: { done: number; total: number }[];
  stepMs?: number;
}

/**
 * Answers the cache settings' commands in the page, as `__cacheCommand`
 * (`undefined` for other commands); channels go through the callbacks the
 * main mock keeps in `__tauriCallbacks`.
 */
export async function mockCache(page: Page, mock: CacheMock | undefined): Promise<void> {
  await page.addInitScript((m: CacheMock | undefined) => {
    // Read when needed: the main mock sets it up after this script.
    const callbacks = () =>
      (window as unknown as { __tauriCallbacks: Map<number, (payload: unknown) => void> })
        .__tauriCallbacks;
    const defaultCacheDir = 'C:\\Users\\Ala\\AppData\\Local\\io.github.lukaszkalnik.sito';
    let cacheSettings: CacheSettingsMock = m?.settings ?? {
      folder: defaultCacheDir,
      location: { kind: 'default' },
      platform: 'windows',
      thumbnailLimitMb: 500,
    };
    const cacheSize = {
      ...(m?.size ?? { thumbnails: 412 * 1024 * 1024, photoData: 18 << 20 }),
    };
    /** Streams the move's progress through `channel`, then resolves with the new settings. */
    const moveCache = (folder: string | null, channel: { id: number }) =>
      new Promise((resolve) => {
        const steps = m?.moveProgress ?? [];
        const every = m?.stepMs ?? 300;
        const send = callbacks().get(channel.id);
        steps.forEach((message, index) =>
          setTimeout(() => send?.({ index, message }), every * (index + 1)),
        );
        setTimeout(
          () => {
            cacheSettings =
              folder === null
                ? { ...cacheSettings, folder: defaultCacheDir, location: { kind: 'default' } }
                : {
                    ...cacheSettings,
                    folder: `${folder}\\Sito-cache`,
                    location: { kind: 'chosen' },
                  };
            resolve(cacheSettings);
          },
          every * (steps.length + 1),
        );
      });
    /** Answers the cache settings' commands; `undefined` for any other command. */
    const cacheCommand = (cmd: string, args: Record<string, unknown>): unknown => {
      switch (cmd) {
        case 'get_cache_settings':
          return cacheSettings;
        case 'measure_cache':
          return { ...cacheSize };
        case 'set_thumbnail_limit':
          cacheSettings = { ...cacheSettings, thumbnailLimitMb: Math.max(50, args.mb as number) };
          return cacheSettings.thumbnailLimitMb;
        case 'clear_thumbnails':
          cacheSize.thumbnails = 0;
          return null;
        case 'clear_photo_data':
          cacheSize.photoData = 0;
          return null;
        case 'check_cache_folder':
          return {
            target: `${args.folder as string}\\Sito-cache`,
            network: false,
            size: cacheSize.thumbnails + cacheSize.photoData,
          };
        case 'move_cache':
          return moveCache(args.folder as string | null, args.onProgress as { id: number });
        case 'reveal_cache_folder':
        case 'cancel_cache_move':
          return null;
        default:
          return undefined;
      }
    };
    Object.defineProperty(window, '__cacheCommand', { value: cacheCommand });
  }, mock);
}
