import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useShortcuts } from '../../shortcuts/useShortcuts';
import {
  canUndo,
  onTimelineChanged,
  rejectErrorCode,
  undoLast,
  type RejectErrorCode,
  type Undone,
} from './api';

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

/** Translation keys of the messages for a failed undo. */
const UNDO_ERROR_KEYS: Readonly<Record<RejectErrorCode, string>> = {
  occupied: 'undo.occupied',
  missing: 'undo.missing',
  missingFromTrash: 'undo.missingFromTrash',
  trashUndoUnsupported: 'undo.trashUndoUnsupported',
  noTrash: 'undo.failed',
  io: 'undo.failed',
  internal: 'undo.failed',
};

/** The translation key of the message for a failed undo. */
function undoErrorKey(error: unknown): string {
  return UNDO_ERROR_KEYS[rejectErrorCode(error)];
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
      const undone = await undoLast();
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
  // Ctrl+Z (Cmd+Z on macOS).
  useShortcuts({ undo: () => void undo() });

  return { canUndo: available, undo, toast, showToast: setToast };
}
