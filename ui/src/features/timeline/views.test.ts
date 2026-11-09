import { describe, expect, it } from 'vitest';

import { parseView, VIEWS } from './views';

describe('views', () => {
  it('lists the series first, then all photos', () => {
    expect(VIEWS).toEqual(['series', 'all']);
  });

  it('parses each view', () => {
    expect(parseView('series')).toBe('series');
    expect(parseView('all')).toBe('all');
  });

  it('rejects an unknown value', () => {
    expect(parseView('focus')).toBeNull();
    expect(parseView('toString')).toBeNull();
  });
});
