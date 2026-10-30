import { useEffect, useRef } from 'react';

import { actionOf, repeatsWhenHeld, type ShortcutAction } from './keymap';

/**
 * What each action does while the component is mounted. `null` means the
 * action is not available right now: its key is then left to the browser.
 */
export type ShortcutHandlers = Partial<Record<ShortcutAction, (() => void) | null>>;

/** Runs the handler of the action whose key is pressed anywhere in the window. */
export function useShortcuts(handlers: ShortcutHandlers): void {
  // The latest handlers, so the listener is attached once and never stale.
  const current = useRef(handlers);
  useEffect(() => {
    current.current = handlers;
  });
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
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
  }, []);
}
