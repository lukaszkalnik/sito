import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import { ReleaseNotes } from './ReleaseNotes';
import './ReleaseNotesDialog.css';

/** Modal dialog with the release notes of a newer version, formatted. */
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
  const text = useRef<HTMLDivElement>(null);

  // The native modal traps focus and closes on Escape.
  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    if (open && !element.open) {
      element.showModal();
      // The dialog stays mounted, so the notes would keep the earlier scroll;
      // a hidden dialog has no layout, so this comes after showing it.
      if (text.current) text.current.scrollTop = 0;
    }
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
      <div
        ref={text}
        className="release-notes-text"
        role="region"
        tabIndex={0}
        aria-label={t('update.notes')}
      >
        <ReleaseNotes notes={notes} />
      </div>
      <button type="button" onClick={onClose}>
        {t('update.close')}
      </button>
    </dialog>
  );
}
