import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import {
  canUndo,
  onTimelineChanged,
  rejectErrorCode,
  undoReject,
  type Undone,
} from '../../api/reject';

/** How long a toast stays on screen. */
export const TOAST_MS = 5000;

/** A short message about the last action, optionally offering "Cofnij". */
export interface ToastMessage {
  text: string;
  /** Show a "Cofnij" button in the toast. */
  offerUndo?: boolean;
  error?: boolean;
}

/**
 * "Cofnij" for rejected photos in one window: whether there is anything to
 * undo (refreshed whenever any window rejects or restores), Ctrl+Z, and the
 * toast reporting the outcome. `onUndone` is told which photo came back.
 */
export function useUndo(onUndone?: (undone: Undone) => void) {
  const { t } = useTranslation();
  const [available, setAvailable] = useState(false);
  const [toast, setToast] = useState<ToastMessage | null>(null);
  const busy = useRef(false);

  useEffect(() => {
    let active = true;
    const refresh = () =>
      void canUndo().then(
        (value) => active && setAvailable(value),
        () => active && setAvailable(false),
      );
    refresh();
    const unlisten = onTimelineChanged(refresh);
    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(null), TOAST_MS);
    return () => clearTimeout(timer);
  }, [toast]);

  const undo = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    try {
      const undone = await undoReject();
      if (undone) {
        setToast({ text: t('undo.done', { name: undone.name }) });
        onUndone?.(undone);
      }
    } catch (error) {
      const code = rejectErrorCode(error);
      const key = code === 'occupied' || code === 'missing' ? `undo.${code}` : 'undo.failed';
      setToast({ text: t(key), error: true });
    } finally {
      busy.current = false;
    }
  }, [onUndone, t]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z') {
        event.preventDefault();
        void undo();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [undo]);

  return { canUndo: available, undo, toast, showToast: setToast };
}
