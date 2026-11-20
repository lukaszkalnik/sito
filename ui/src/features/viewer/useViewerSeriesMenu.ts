import { useCallback, useRef, useState } from 'react';

import type { ShortcutAction } from '../../shortcuts/keymap';
import { useShortcuts } from '../../shortcuts/useShortcuts';
import type { ToastMessage } from '../reject/useUndo';
import type { EditAction, EditOption } from '../series-edit/editOptions';
import type { OpenMenu } from '../series-edit/SeriesMenu';
import { useEditSeries } from '../series-edit/useEditSeries';
import type { MenuRequest } from '../timeline/menuRequest';
import type { ViewerPhoto } from './api';
import { afterRemove, viewerEditOptions } from './viewerEditOptions';

/** The keys that make an edit of the shown photo directly, shown in the menu. */
export const EDIT_SHORTCUTS: Readonly<Partial<Record<EditAction, ShortcutAction>>> = {
  cut: 'cutSeries',
  remove: 'removeFromSeries',
};

interface ViewerSeriesMenuOptions {
  photo: ViewerPhoto;
  /** Time gap of the series, in seconds. */
  gap: number;
  showToast: (message: ToastMessage) => void;
  /** Shows the photo with the given id. */
  onShow: (id: number) => void;
  /** Points out photos of the strip, e.g. those a restore brought back. */
  onPulse: (ids: readonly number[]) => void;
}

/** Opens the menu of the focused strip thumbnail, as the Menu key does by itself. */
function openMenuOfStripItem(): boolean {
  const focused = document.activeElement;
  if (!(focused instanceof HTMLElement) || !focused.closest('.viewer-strip')) return false;
  focused.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
  return true;
}

/** The option of `action` among `options`, if offered. */
function optionOf(options: readonly EditOption[], action: EditAction): EditOption | undefined {
  return options.find((option) => option.action === action);
}

/**
 * The open edit menu: `show` opens it with `options` at a window position
 * (nothing without options), `close` closes it, giving the focus back to
 * the element focused when it opened if `returnFocus`.
 */
function useMenuState() {
  const [menu, setMenu] = useState<OpenMenu | null>(null);
  const opener = useRef<HTMLElement | null>(null);
  const show = useCallback((options: EditOption[], x: number, y: number) => {
    if (options.length === 0) return;
    const focused = document.activeElement;
    opener.current = focused instanceof HTMLElement && focused !== document.body ? focused : null;
    setMenu({ x, y, options });
  }, []);
  const close = useCallback((returnFocus: boolean) => {
    setMenu(null);
    if (returnFocus) opener.current?.focus();
  }, []);
  return { menu, show, close };
}

/**
 * Makes an edit, one at a time, and follows it in the viewer: after taking
 * out the shown photo it shows the next photo of the strip (the viewer stays
 * in the series); after a restore the strip points out the changed photos.
 */
function useRunEdit({ photo, gap, showToast, onShow, onPulse }: ViewerSeriesMenuOptions) {
  const edit = useEditSeries(gap, showToast);
  const editing = useRef(false);
  return async (option: EditOption) => {
    if (editing.current) return;
    editing.current = true;
    try {
      const edited = await edit(option);
      if (!edited) return;
      if (option.action === 'restore') onPulse(edited.changed);
      if (option.action === 'remove' && option.edit.photo === photo.entry.id) {
        const next = afterRemove(photo.series, photo.entry.id);
        if (next !== null) onShow(next);
      }
    } finally {
      editing.current = false;
    }
  };
}

/**
 * The viewer's series edits (see {@link viewerEditOptions}): a menu opened
 * from the bar button (`openBelow`, `null` with nothing to edit), a context
 * menu (`open`), Shift+F10 (on a focused strip thumbnail, else the shown
 * photo, below the bar button) and the keys P and W on the shown photo.
 */
export function useViewerSeriesMenu(options: ViewerSeriesMenuOptions) {
  const { photo } = options;
  const { menu, show, close } = useMenuState();
  const run = useRunEdit(options);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const shownOptions = viewerEditOptions(photo, photo.entry.id);

  const open = (request: MenuRequest) => {
    if (request.target.kind !== 'photo') return;
    show(viewerEditOptions(photo, request.target.id), request.x, request.y);
  };
  const openBelow = (anchor: HTMLElement) => {
    anchor.focus();
    const rect = anchor.getBoundingClientRect();
    show(shownOptions, rect.left, rect.bottom);
  };
  const choose = (option: EditOption) => {
    close(true);
    void run(option);
  };
  const direct = (action: EditAction) => {
    const option = optionOf(shownOptions, action);
    return option ? () => void run(option) : null;
  };
  useShortcuts({
    menu: () => {
      if (!openMenuOfStripItem() && buttonRef.current) openBelow(buttonRef.current);
    },
    cutSeries: direct('cut'),
    removeFromSeries: direct('remove'),
  });

  return {
    menu,
    open,
    close,
    choose,
    buttonRef,
    openBelow: shownOptions.length > 0 ? openBelow : null,
  };
}
