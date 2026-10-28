import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { canUndo, onTimelineChanged, rejectErrorCode, undoReject, type Undone } from './api';

/** How long a toast stays on screen. */
export const TOAST_MS = 5000;

/**
 * A short message about the last action: plain `info`, `undoable` (with a
 * "Cofnij" button) or an `error` (announced as an alert).
 */
export interface ToastMessage {
  text: string;
  kind: 'info' | 'undoable' | 'error';
}

/** Whether there is anything to undo, refreshed whenever any window rejects or restores. */
function useUndoAvailable(): boolean {
  const [available, setAvailable] = useState(false);
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
  return available;
}

/** A toast message that disappears after {@link TOAST_MS}. */
function useToast() {
  const [toast, setToast] = useState<ToastMessage | null>(null);
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(null), TOAST_MS);
    return () => clearTimeout(timer);
  }, [toast]);
  return [toast, setToast] as const;
}

/** Calls `undo` on Ctrl+Z (Cmd+Z on macOS). */
function useUndoShortcut(undo: () => Promise<void>): void {
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
}

/** The translation key of the message for a failed undo. */
function undoErrorKey(error: unknown): string {
  const code = rejectErrorCode(error);
  return code === 'occupied' || code === 'missing' ? `undo.${code}` : 'undo.failed';
}

/**
 * "Cofnij" for rejected photos in one window: whether there is anything to
 * undo, Ctrl+Z, and the toast reporting the outcome. `onUndone` is told which
 * photo came back.
 */
export function useUndo(onUndone?: (undone: Undone) => void) {
  const { t } = useTranslation();
  const available = useUndoAvailable();
  const [toast, setToast] = useToast();
  const busy = useRef(false);

  const undo = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    try {
      const undone = await undoReject();
      if (undone) {
        setToast({ text: t('undo.done', { name: undone.name }), kind: 'info' });
        onUndone?.(undone);
      }
    } catch (error) {
      setToast({ text: t(undoErrorKey(error)), kind: 'error' });
    } finally {
      busy.current = false;
    }
  }, [onUndone, t, setToast]);
  useUndoShortcut(undo);

  return { canUndo: available, undo, toast, showToast: setToast };
}
