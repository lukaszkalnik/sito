import { useEffect, useRef } from 'react';

import { actionOf, repeatsWhenHeld, type ShortcutAction } from './keymap';

/**
 * What each action does while the component is mounted. `null` means the
 * action is not available right now: its key is then left to the browser.
 */
export type ShortcutHandlers = Partial<Record<ShortcutAction, (() => void) | null>>;

/**
 * Which keys act: `page` shortcuts act unless a `modal` layer (e.g. an open
 * menu) has a handler available, which then leaves only the keys of the
 * modal layers acting, so Escape closes the menu and not the window behind it.
 */
export type ShortcutLayer = 'page' | 'modal';

/** Number of mounted modal layers with a handler available, in this window. */
let activeModalLayers = 0;

/** Counts the modal layer in while it is `active`. */
function useModalLayer(active: boolean) {
  useEffect(() => {
    if (!active) return;
    activeModalLayers++;
    return () => {
      activeModalLayers--;
    };
  }, [active]);
}

/**
 * Runs the handler of the action whose key is pressed anywhere in the
 * window, unless a modal layer holds the keys (see {@link ShortcutLayer}).
 */
export function useShortcuts(handlers: ShortcutHandlers, layer: ShortcutLayer = 'page'): void {
  // The latest handlers, so the listener is attached once and never stale.
  const current = useRef(handlers);
  useEffect(() => {
    current.current = handlers;
  });
  useModalLayer(layer === 'modal' && Object.values(handlers).some(Boolean));
  const modal = layer === 'modal';
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!modal && activeModalLayers > 0) return;
      const available = (Object.keys(current.current) as ShortcutAction[]).filter(
        (action) => current.current[action],
      );
      const action = actionOf(event, available);
      const handler = action && current.current[action];
      if (!handler) return;
      event.preventDefault();
      if (event.repeat && !repeatsWhenHeld(action)) return;
      handler();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [modal]);
}
