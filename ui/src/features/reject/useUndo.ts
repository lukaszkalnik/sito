import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useShortcuts } from '../../shortcuts/useShortcuts';
import {
  onTimelineChanged,
  rejectErrorCode,
  undoLast,
  undoState,
  type RejectErrorCode,
  type Undone,
  type UndoState,
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

/** What "Cofnij" can undo, refreshed whenever any window rejects or restores a photo. */
function useUndoState(): UndoState {
  const [state, setState] = useState<UndoState>('none');
  useEffect(() => {
    let active = true;
    const refresh = () =>
      void undoState().then(
        (value) => active && setState(value),
        () => active && setState('none'),
      );
    refresh();
    const unlisten = onTimelineChanged(refresh);
    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);
  return state;
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
 * "Cofnij" in one window: what there is to undo, Ctrl+Z, and the toast
 * reporting the outcome. `onUndone` is told which photo came back. While only
 * rejections into a trash that cannot be restored from are left, undoing
 * only explains that in the toast.
 */
export function useUndo(onUndone?: (undone: Undone) => void) {
  const { t } = useTranslation();
  const state = useUndoState();
  const [toast, setToast] = useToast();
  const busy = useRef(false);

  const undo = useCallback(async () => {
    if (busy.current) return;
    if (state === 'trashOnly') {
      setToast({ text: t('undo.trashUndoUnsupported'), kind: 'error' });
      return;
    }
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
  }, [onUndone, t, setToast, state]);
  // Ctrl+Z (Cmd+Z on macOS).
  useShortcuts({ undo: () => void undo() });

  return { undoState: state, undo, toast, showToast: setToast };
}
