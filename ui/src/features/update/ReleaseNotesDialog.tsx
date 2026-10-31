import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import './ReleaseNotesDialog.css';

/** Modal dialog with the release notes of a newer version, as plain text. */
export function ReleaseNotesDialog({
  open,
  version,
  notes,
  onClose,
}: {
  open: boolean;
  version: string;
  notes: string;
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
    <dialog
      ref={dialog}
      className="release-notes"
      aria-labelledby="release-notes-title"
      onClose={onClose}
    >
      <h2 id="release-notes-title">{t('update.notesTitle', { version })}</h2>
      {/* Focusable, so long notes can be scrolled with the keyboard. */}
      <pre className="release-notes-text" tabIndex={0} aria-label={t('update.notes')}>
        {notes}
      </pre>
      <button type="button" onClick={onClose}>
        {t('update.close')}
      </button>
    </dialog>
  );
}
