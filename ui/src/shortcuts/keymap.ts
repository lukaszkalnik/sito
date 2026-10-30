/** Every keyboard action of the app. */
export type ShortcutAction =
  | 'previous'
  | 'next'
  | 'close'
  | 'reject'
  | 'accept'
  | 'undo'
  | 'zoomIn'
  | 'zoomOut'
  | 'zoomFit'
  | 'zoomActual'
  | 'panLeft'
  | 'panRight'
  | 'panUp'
  | 'panDown';

/**
 * A key combination: `key` as in `KeyboardEvent.key` (letters lower case),
 * pressed alone, with Shift, or with the platform's primary modifier (Ctrl,
 * Cmd on macOS).
 */
export interface Shortcut {
  key: string;
  modifier: 'none' | 'shift' | 'primary';
}

/** The key of every action. Keys are unique; a unit test checks it. */
export const KEYMAP: Readonly<Record<ShortcutAction, Shortcut>> = {
  previous: { key: 'ArrowLeft', modifier: 'none' },
  next: { key: 'ArrowRight', modifier: 'none' },
  close: { key: 'Escape', modifier: 'none' },
  reject: { key: 'Delete', modifier: 'none' },
  accept: { key: 'a', modifier: 'none' },
  undo: { key: 'z', modifier: 'primary' },
  zoomIn: { key: '+', modifier: 'none' },
  zoomOut: { key: '-', modifier: 'none' },
  zoomFit: { key: '0', modifier: 'none' },
  zoomActual: { key: '1', modifier: 'none' },
  panLeft: { key: 'ArrowLeft', modifier: 'shift' },
  panRight: { key: 'ArrowRight', modifier: 'shift' },
  panUp: { key: 'ArrowUp', modifier: 'shift' },
  panDown: { key: 'ArrowDown', modifier: 'shift' },
};

/**
 * Whether Shift is as `shortcut` needs it. A printable character ignores
 * Shift, because Shift is part of typing it (`+` is Shift+`=` on many
 * layouts); a named key such as `ArrowLeft` needs Shift exactly when the
 * shortcut has it.
 */
function shiftMatches(event: KeyboardEvent, shortcut: Shortcut): boolean {
  if (shortcut.modifier === 'shift') return event.shiftKey;
  return shortcut.key.length === 1 || !event.shiftKey;
}

/** Whether `event` is the key combination `shortcut`. */
export function matches(event: KeyboardEvent, shortcut: Shortcut): boolean {
  const primary = event.ctrlKey || event.metaKey;
  if (primary !== (shortcut.modifier === 'primary')) return false;
  if (!shiftMatches(event, shortcut)) return false;
  return event.key.toLowerCase() === shortcut.key.toLowerCase();
}

/** The action among `actions` whose shortcut `event` is, if any. */
export function actionOf(
  event: KeyboardEvent,
  actions: readonly ShortcutAction[],
): ShortcutAction | undefined {
  return actions.find((action) => matches(event, KEYMAP[action]));
}

/**
 * Actions that a held key (auto-repeat) does not repeat: each press does one.
 * Rejecting and undoing move files, so holding Delete must not reject photo
 * after photo; holding A would toggle the mark on and off; closing twice
 * would fail.
 */
const ONCE_PER_PRESS: ReadonlySet<ShortcutAction> = new Set(['reject', 'accept', 'undo', 'close']);

/** Whether a key held down repeats `action`. */
export function repeatsWhenHeld(action: ShortcutAction): boolean {
  return !ONCE_PER_PRESS.has(action);
}
