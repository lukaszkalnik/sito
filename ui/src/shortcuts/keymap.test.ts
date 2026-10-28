import { describe, expect, it } from 'vitest';

import { actionOf, KEYMAP, matches } from './keymap';

function press(key: string, init: KeyboardEventInit = {}): KeyboardEvent {
  return new KeyboardEvent('keydown', { key, ...init });
}

describe('KEYMAP', () => {
  it('gives every action its own key', () => {
    const combos = Object.values(KEYMAP).map((s) => `${s.modifier}+${s.key.toLowerCase()}`);
    expect(new Set(combos).size).toBe(combos.length);
  });
});

describe('matches', () => {
  it('needs the primary modifier exactly when the shortcut has it', () => {
    expect(matches(press('z', { ctrlKey: true }), KEYMAP.undo)).toBe(true);
    expect(matches(press('z', { metaKey: true }), KEYMAP.undo)).toBe(true);
    expect(matches(press('z'), KEYMAP.undo)).toBe(false);
    expect(matches(press('Delete', { ctrlKey: true }), KEYMAP.reject)).toBe(false);
  });

  it('ignores the case of letters, e.g. with Caps Lock', () => {
    expect(matches(press('Z', { ctrlKey: true }), KEYMAP.undo)).toBe(true);
  });
});

describe('actionOf', () => {
  it('finds only among the given actions', () => {
    expect(actionOf(press('Delete'), ['reject', 'close'])).toBe('reject');
    expect(actionOf(press('Delete'), ['close'])).toBeUndefined();
  });
});
