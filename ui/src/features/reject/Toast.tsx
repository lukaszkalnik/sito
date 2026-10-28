import { useTranslation } from 'react-i18next';

import type { ToastMessage } from './useUndo';
import './Toast.css';

/** A short message at the bottom of the window, with an optional "Cofnij". */
export function Toast({ message, onUndo }: { message: ToastMessage | null; onUndo: () => void }) {
  const { t } = useTranslation();
  return (
    // Always rendered, so screen readers announce new messages.
    <div className="toast-area" aria-live="polite">
      {message && (
        <div
          className={`toast${message.kind === 'error' ? ' toast--error' : ''}`}
          data-testid="toast"
          role={message.kind === 'error' ? 'alert' : undefined}
        >
          <span>{message.text}</span>
          {message.kind === 'undoable' && (
            <button type="button" onClick={onUndo}>
              {t('undo.button')}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
