import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { clampGap, DEFAULT_GAP, loadGap, MAX_GAP, MIN_GAP, saveGap } from './gapSetting';

describe('gapSetting', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('uses the default for a folder without a stored gap', () => {
    expect(loadGap('D:\\Desktop')).toBe(DEFAULT_GAP);
  });

  it('remembers the gap per folder', () => {
    saveGap('D:\\Desktop\\Złote gody', 4);
    saveGap('D:\\Zdjęcia', 25);

    expect(loadGap('D:\\Desktop\\Złote gody')).toBe(4);
    expect(loadGap('D:\\Zdjęcia')).toBe(25);
    expect(loadGap('D:\\Desktop')).toBe(DEFAULT_GAP);
  });

  it('ignores broken stored data', () => {
    localStorage.setItem('sito.timeGap', '{not json');
    expect(loadGap('D:\\Zdjęcia')).toBe(DEFAULT_GAP);

    localStorage.setItem('sito.timeGap', JSON.stringify({ 'D:\\Zdjęcia': 'fast' }));
    expect(loadGap('D:\\Zdjęcia')).toBe(DEFAULT_GAP);

    saveGap('D:\\Zdjęcia', 7);
    expect(loadGap('D:\\Zdjęcia')).toBe(7);
  });

  it.each([
    [0, MIN_GAP],
    [-5, MIN_GAP],
    [500, MAX_GAP],
    [4.6, 5],
    [Number.NaN, DEFAULT_GAP],
    ['10', DEFAULT_GAP],
  ])('clamps %j to %i', (value, expected) => {
    expect(clampGap(value)).toBe(expected);
  });

  it('keeps working when storage throws', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('denied');
    });
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('denied');
    });

    expect(() => saveGap('D:\\Zdjęcia', 5)).not.toThrow();
    expect(loadGap('D:\\Zdjęcia')).toBe(DEFAULT_GAP);
  });
});
