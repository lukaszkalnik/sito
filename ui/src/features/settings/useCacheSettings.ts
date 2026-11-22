import { useCallback, useEffect, useState } from 'react';

import {
  getCacheSettings,
  measureCache,
  onShowCacheSettings,
  setThumbnailLimit,
  type CacheSettings,
  type CacheSize,
} from './api';

/** The settings, while loading, after a failure, or loaded. */
export type SettingsState =
  | { readonly kind: 'loading' }
  | { readonly kind: 'failed' }
  | { readonly kind: 'loaded'; readonly settings: CacheSettings };

/** The space the cache takes: being measured, measured, or not measurable. */
export type SizeState =
  | { readonly kind: 'measuring' }
  | { readonly kind: 'failed' }
  | { readonly kind: 'measured'; readonly size: CacheSize };

/** Calls `onShow` whenever the menu asks for the cache settings. */
function useMenuRequest(onShow: () => void) {
  useEffect(() => {
    const unlisten = onShowCacheSettings(onShow).catch(
      // Outside the app (e.g. a plain browser) there are no events.
      () => () => {},
    );
    return () => void unlisten.then((stop) => stop());
  }, [onShow]);
}

/**
 * The cache settings dialog's data: whether it is open, the settings (loaded
 * at start, for the fallback notice, and again on every opening) and the
 * cache's size (measured on every opening and by {@link remeasure}).
 */
export function useCacheSettings() {
  const [open, setOpen] = useState(false);
  const [settings, setSettings] = useState<SettingsState>({ kind: 'loading' });
  const [size, setSize] = useState<SizeState>({ kind: 'measuring' });

  const reload = useCallback(() => {
    getCacheSettings().then(
      (loaded) => setSettings({ kind: 'loaded', settings: loaded }),
      (err: unknown) => {
        console.error('Cannot load the cache settings', err);
        setSettings({ kind: 'failed' });
      },
    );
  }, []);
  const remeasure = useCallback(() => {
    setSize({ kind: 'measuring' });
    measureCache().then(
      (measured) => setSize({ kind: 'measured', size: measured }),
      () => setSize({ kind: 'failed' }),
    );
  }, []);
  const show = useCallback(() => {
    setOpen(true);
    reload();
    remeasure();
  }, [reload, remeasure]);

  useEffect(reload, [reload]);
  useMenuRequest(show);

  const setLimit = (mb: number) => {
    setThumbnailLimit(mb).then(
      (applied) => {
        setSettings((current) =>
          current.kind === 'loaded'
            ? { kind: 'loaded', settings: { ...current.settings, thumbnailLimitMb: applied } }
            : current,
        );
        remeasure();
      },
      (err: unknown) => console.error('Cannot set the thumbnail limit', err),
    );
  };
  const applied = (next: CacheSettings) => setSettings({ kind: 'loaded', settings: next });

  return { open, show, close: () => setOpen(false), settings, size, remeasure, setLimit, applied };
}
