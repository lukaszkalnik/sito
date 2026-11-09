import { describe, expect, it } from 'vitest';

import { splitPathTail } from './pathTail';

describe('splitPathTail', () => {
  it('splits a Windows path before its last folder', () => {
    expect(splitPathTail('C:\\Zdjęcia\\Złote gody')).toEqual({
      head: 'C:\\Zdjęcia\\',
      tail: 'Złote gody',
    });
  });

  it('splits a macOS path before its last folder', () => {
    expect(splitPathTail('/Volumes/Extreme SSD')).toEqual({
      head: '/Volumes/',
      tail: 'Extreme SSD',
    });
  });

  it('keeps a root or a name without separators in one part', () => {
    expect(splitPathTail('C:\\')).toEqual({ head: 'C:\\', tail: '' });
    expect(splitPathTail('/')).toEqual({ head: '/', tail: '' });
    expect(splitPathTail('Zdjęcia')).toEqual({ head: '', tail: 'Zdjęcia' });
  });
});
