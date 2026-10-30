import { useRef } from 'react';
import { useTranslation } from 'react-i18next';

import type { ToastMessage } from '../reject/useUndo';
import { acceptPhoto, unacceptPhoto } from './api';

/**
 * Accepts photo `id`, or takes the mark off when `acceptedIds` has it, and
 * reports the outcome in a toast. Ignores calls while one is running.
 */
export function useToggleAccept(
  acceptedIds: ReadonlySet<number>,
  showToast: (message: ToastMessage) => void,
) {
  const { t } = useTranslation();
  const busy = useRef(false);
  return async (id: number) => {
    if (busy.current) return;
    busy.current = true;
    try {
      if (acceptedIds.has(id)) {
        const { name } = await unacceptPhoto(id);
        showToast({ text: t('accept.undone', { name }), kind: 'undoable' });
      } else {
        const { name } = await acceptPhoto(id);
        showToast({ text: t('accept.done', { name }), kind: 'undoable' });
      }
    } catch (error) {
      console.error('Cannot change the accepted mark', error);
      showToast({ text: t('accept.failed'), kind: 'error' });
    } finally {
      busy.current = false;
    }
  };
}
