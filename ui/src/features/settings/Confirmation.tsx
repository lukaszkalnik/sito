import { useId, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

/** A question inside the settings dialog with a confirming button and „Anuluj”. */
export function Confirmation({
  question,
  confirmLabel,
  onConfirm,
  onCancel,
  children,
}: {
  question: string;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
  /** More text below the question, e.g. a warning. */
  children?: ReactNode;
}) {
  const { t } = useTranslation();
  const id = useId();
  return (
    <div className="cache-confirmation" role="alertdialog" aria-labelledby={id}>
      <p id={id}>{question}</p>
      {children}
      <div className="cache-buttons">
        <button type="button" onClick={onConfirm}>
          {confirmLabel}
        </button>
        <button type="button" onClick={onCancel}>
          {t('cache.cancel')}
        </button>
      </div>
    </div>
  );
}
