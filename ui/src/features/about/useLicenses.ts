import { useEffect, useState } from 'react';

import { appVersion, loadNotices, onShowLicenses } from './api';

/** The notices, while loading, after a failure, or loaded with the app version. */
export type LicensesState =
  | { readonly status: 'loading' }
  | { readonly status: 'error' }
  | { readonly status: 'ready'; readonly version: string; readonly text: string };

/**
 * The licenses dialog: opened from the menu, closed by the dialog. The notices
 * load when it first opens and are kept; after a failure, opening it again
 * retries.
 */
export function useLicensesDialog() {
  const [open, setOpen] = useState(false);
  const [state, setState] = useState<LicensesState>({ status: 'loading' });

  useEffect(() => {
    const unlisten = onShowLicenses(() => {
      setOpen(true);
      setState((current) => (current.status === 'error' ? { status: 'loading' } : current));
    }).catch(
      // Outside the app (e.g. a plain browser) there are no events.
      () => () => {},
    );
    return () => void unlisten.then((stop) => stop());
  }, []);

  const load = open && state.status === 'loading';
  useEffect(() => {
    if (!load) return;
    let active = true;
    Promise.all([appVersion(), loadNotices()]).then(
      ([version, text]) => active && setState({ status: 'ready', version, text }),
      (err: unknown) => {
        console.error('Cannot load the licenses', err);
        if (active) setState({ status: 'error' });
      },
    );
    return () => {
      active = false;
    };
  }, [load]);

  return { open, close: () => setOpen(false), state };
}
