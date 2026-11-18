import { type RefObject, useEffect, useLayoutEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import { onFirstClick } from '../../clicks/firstClick';
import { useShortcuts } from '../../shortcuts/useShortcuts';
import type { EditOption } from './editOptions';
import './SeriesMenu.css';

/** An open edit menu: where it is and what it offers. */
export interface OpenMenu {
  readonly x: number;
  readonly y: number;
  readonly options: readonly EditOption[];
}

interface SeriesMenuProps {
  menu: OpenMenu | null;
  onChoose: (option: EditOption) => void;
  /** Closes the menu; `returnFocus` when focus should go back where it was. */
  onClose: (returnFocus: boolean) => void;
}

/** Space kept between the menu and the window's edges, in pixels. */
const EDGE = 8;

/** Moves the menu inside the window if it would stick out. */
function useKeptInWindow(ref: RefObject<HTMLUListElement | null>, menu: OpenMenu | null) {
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element || !menu) return;
    const { width, height } = element.getBoundingClientRect();
    const left = Math.max(EDGE, Math.min(menu.x, window.innerWidth - width - EDGE));
    const top = Math.max(EDGE, Math.min(menu.y, window.innerHeight - height - EDGE));
    element.style.left = `${left}px`;
    element.style.top = `${top}px`;
  }, [ref, menu]);
}

/** Closes the menu on a press anywhere outside it. */
function useClosedOutside(
  ref: RefObject<HTMLUListElement | null>,
  open: boolean,
  onClose: (returnFocus: boolean) => void,
) {
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!ref.current?.contains(event.target as Node)) onClose(false);
    };
    document.addEventListener('pointerdown', onPointerDown, true);
    return () => document.removeEventListener('pointerdown', onPointerDown, true);
  }, [ref, open, onClose]);
}

/** Moves the focus `step` items through the menu, wrapping around. */
function moveFocus(menu: HTMLUListElement | null, step: number) {
  if (!menu) return;
  const items = Array.from(menu.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'));
  const current = items.findIndex((item) => item === document.activeElement);
  const next = (current + step + items.length) % items.length;
  items[next]?.focus();
}

/** The context menu with the series edits: arrow keys move, Escape closes. */
export function SeriesMenu({ menu, onChoose, onClose }: SeriesMenuProps) {
  const { t } = useTranslation();
  const ref = useRef<HTMLUListElement>(null);
  useKeptInWindow(ref, menu);
  useClosedOutside(ref, menu !== null, onClose);
  useEffect(() => {
    if (menu) ref.current?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
  }, [menu]);
  const open = menu !== null;
  useShortcuts({
    close: open ? () => onClose(true) : null,
    menuUp: open ? () => moveFocus(ref.current, -1) : null,
    menuDown: open ? () => moveFocus(ref.current, 1) : null,
  });
  if (!menu) return null;
  return (
    <ul
      ref={ref}
      className="series-menu"
      role="menu"
      aria-label={t('seriesEdit.menu')}
      style={{ left: menu.x, top: menu.y }}
      onBlur={(event) => {
        // Only focus moving to another element closes it: WebKit does not
        // focus a clicked button, so pressing an item blurs the menu with no
        // new focus, and closing then would swallow the click.
        const to = event.relatedTarget;
        if (to instanceof Node && !ref.current?.contains(to)) onClose(false);
      }}
    >
      {menu.options.map((option) => (
        <li key={option.action} role="none">
          <button
            type="button"
            role="menuitem"
            className="series-menu-item"
            onClick={onFirstClick(() => onChoose(option))}
          >
            {t(`seriesEdit.${option.action}`)}
          </button>
        </li>
      ))}
    </ul>
  );
}
