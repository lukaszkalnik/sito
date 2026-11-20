import { useCallback, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useShortcuts } from '../../shortcuts/useShortcuts';
import type { ToastMessage } from '../reject/useUndo';
import type { MenuRequest } from '../timeline/menuRequest';
import { editErrorCode, editSeries, type EditErrorCode } from './api';
import { editOptionsFor, type Arranged, type EditAction, type EditOption } from './editOptions';
import type { OpenMenu } from './SeriesMenu';

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
 * Opens a context menu on the focused element, as the Menu key does by
 * itself: the grid's elements open their menus on `contextmenu`.
 */
function openMenuOfFocused() {
  const focused = document.activeElement;
  if (focused instanceof HTMLElement && focused !== document.body) {
    focused.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
  }
}

/**
 * The edit menu of the grid showing `arranged` for gap `gap` (seconds):
 * `open` shows the edits for a menu request (nothing when there are none),
 * `choose` makes one and reports it in a toast with „Cofnij”. Focus goes back to where it
 * was when the menu closes from the keyboard. Shift+F10 opens the menu of
 * the focused photo.
 */
export function useSeriesMenu(
  arranged: Arranged | null,
  gap: number,
  showToast: (message: ToastMessage) => void,
) {
  const { t } = useTranslation();
  const [menu, setMenu] = useState<OpenMenu | null>(null);
  const opener = useRef<HTMLElement | null>(null);
  useShortcuts({ menu: arranged ? openMenuOfFocused : null });

  const open = useCallback(
    (request: MenuRequest) => {
      const options = arranged ? editOptionsFor(arranged, request.target) : [];
      if (options.length === 0) return;
      const focused = document.activeElement;
      opener.current = focused instanceof HTMLElement ? focused : null;
      setMenu({ x: request.x, y: request.y, options });
    },
    [arranged],
  );

  const close = useCallback((returnFocus: boolean) => {
    setMenu(null);
    if (returnFocus) opener.current?.focus();
  }, []);

  const choose = useCallback(
    async (option: EditOption) => {
      close(true);
      try {
        const { name } = await editSeries(option.edit, gap);
        showToast({ text: t(DONE_KEYS[option.action], { name }), kind: 'undoable' });
      } catch (error) {
        console.error('Cannot edit the series', error);
        showToast({ text: t(ERROR_KEYS[editErrorCode(error)]), kind: 'error' });
      }
    },
    [close, gap, showToast, t],
  );

  return { menu, open, close, choose };
}
