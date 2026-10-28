import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import type { LicensesState } from './useLicenses';
import './LicensesDialog.css';

function LicensesBody({ state }: { state: LicensesState }) {
  const { t } = useTranslation();
  switch (state.status) {
    case 'loading':
      return <p role="status">{t('about.loading')}</p>;
    case 'error':
      return <p role="alert">{t('about.loadError')}</p>;
    case 'ready':
      return (
        <>
          <p>{t('about.intro', { version: state.version })}</p>
          {/* Focusable, so the long text can be scrolled with the keyboard. */}
          <pre className="licenses-text" tabIndex={0} aria-label={t('about.text')}>
            {state.text}
          </pre>
        </>
      );
  }
}

/** Modal dialog with Sito's license and the third-party notices. */
export function LicensesDialog({
  open,
  state,
  onClose,
}: {
  open: boolean;
  state: LicensesState;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const dialog = useRef<HTMLDialogElement>(null);

  // The native modal traps focus and closes on Escape.
  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    if (open && !element.open) element.showModal();
    if (!open && element.open) element.close();
  }, [open]);

  return (
    <dialog ref={dialog} className="licenses" aria-labelledby="licenses-title" onClose={onClose}>
      <h2 id="licenses-title">{t('about.licenses')}</h2>
      <LicensesBody state={state} />
      <button type="button" onClick={onClose}>
        {t('about.close')}
      </button>
    </dialog>
  );
}
