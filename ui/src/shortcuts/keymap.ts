/** Every keyboard action of the app. */
export type ShortcutAction = 'previous' | 'next' | 'close' | 'reject' | 'undo';

/**
 * A key combination: `key` as in `KeyboardEvent.key` (letters lower case),
 * pressed alone or with the platform's primary modifier (Ctrl, Cmd on macOS).
 */
export interface Shortcut {
  key: string;
  modifier: 'none' | 'primary';
}

/** The key of every action. Keys are unique; a unit test checks it. */
export const KEYMAP: Readonly<Record<ShortcutAction, Shortcut>> = {
  previous: { key: 'ArrowLeft', modifier: 'none' },
  next: { key: 'ArrowRight', modifier: 'none' },
  close: { key: 'Escape', modifier: 'none' },
  reject: { key: 'Delete', modifier: 'none' },
  undo: { key: 'z', modifier: 'primary' },
};

/** Whether `event` is the key combination `shortcut`. */
export function matches(event: KeyboardEvent, shortcut: Shortcut): boolean {
  const primary = event.ctrlKey || event.metaKey;
  if (primary !== (shortcut.modifier === 'primary')) return false;
  return event.key.toLowerCase() === shortcut.key.toLowerCase();
}

/** The action among `actions` whose shortcut `event` is, if any. */
export function actionOf(
  event: KeyboardEvent,
  actions: readonly ShortcutAction[],
): ShortcutAction | undefined {
  return actions.find((action) => matches(event, KEYMAP[action]));
}
