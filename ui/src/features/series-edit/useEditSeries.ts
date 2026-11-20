import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';

import type { ToastMessage } from '../reject/useUndo';
import { editErrorCode, editSeries, type EditErrorCode, type Edited } from './api';
import type { EditAction, EditOption } from './editOptions';

/** Translation keys of the toast after each edit. */
const DONE_KEYS: Readonly<Record<EditAction, string>> = {
  cut: 'seriesEdit.cutDone',
  remove: 'seriesEdit.removeDone',
  joinPrevious: 'seriesEdit.joinDone',
  joinNext: 'seriesEdit.joinDone',
  ungroup: 'seriesEdit.ungroupDone',
  restore: 'seriesEdit.restoreDone',
};

/** Translation keys of the toast after a failed edit. */
const ERROR_KEYS: Readonly<Record<EditErrorCode, string>> = {
  seriesChanged: 'seriesEdit.seriesChanged',
  internal: 'seriesEdit.failed',
};

/**
 * Makes series edits for gap `gap` (seconds) and reports each in a toast:
 * with „Cofnij” once it is made, or the error. `edit` resolves to the
 * outcome, or `null` if the edit failed.
 */
export function useEditSeries(gap: number, showToast: (message: ToastMessage) => void) {
  const { t } = useTranslation();
  return useCallback(
    async (option: EditOption): Promise<Edited | null> => {
      try {
        const edited = await editSeries(option.edit, gap);
        showToast({ text: t(DONE_KEYS[option.action], { name: edited.name }), kind: 'undoable' });
        return edited;
      } catch (error) {
        console.error('Cannot edit the series', error);
        showToast({ text: t(ERROR_KEYS[editErrorCode(error)]), kind: 'error' });
        return null;
      }
    },
    [gap, showToast, t],
  );
}
