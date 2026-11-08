import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import './TrashConfirmDialog.css';

/**
 * Modal question before rejected photos start going into the system trash.
 * "Anuluj" comes first, so it has the focus; Escape also cancels.
 */
export function TrashConfirmDialog({
  open,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  onConfirm: () => void;
  onCancel: () => void;
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
      className="trash-confirm"
      aria-labelledby="trash-confirm-title"
      aria-describedby="trash-confirm-text"
      // Escape fires `cancel` at once but `close` only in a later task, after
      // which the dialog may already be open again: cancel on `cancel`, and on
      // `close` only if the dialog is still closed (it can close without `cancel`).
      onCancel={onCancel}
      onClose={() => {
        if (!dialog.current?.open) onCancel();
      }}
    >
      <h2 id="trash-confirm-title">{t('destination.confirmTitle')}</h2>
      <p id="trash-confirm-text">{t('destination.confirmText')}</p>
      <div className="trash-confirm-buttons">
        <button type="button" onClick={onCancel}>
          {t('destination.cancel')}
        </button>
        <button type="button" className="trash-confirm-yes" onClick={onConfirm}>
          {t('destination.confirm')}
        </button>
      </div>
    </dialog>
  );
}
