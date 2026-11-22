import { describe, expect, it } from 'vitest';

import { finishedLimit, MIN_LIMIT_MB } from './cacheLimit';

describe('finishedLimit', () => {
  it('keeps a whole number of MB from the least up', () => {
    expect(finishedLimit('800', 500)).toBe(800);
    expect(finishedLimit(String(MIN_LIMIT_MB), 500)).toBe(MIN_LIMIT_MB);
    expect(finishedLimit('20000', 500)).toBe(20000);
  });

  it('raises an empty or smaller value to the least', () => {
    expect(finishedLimit('', 500)).toBe(MIN_LIMIT_MB);
    expect(finishedLimit('  ', 500)).toBe(MIN_LIMIT_MB);
    expect(finishedLimit('10', 500)).toBe(MIN_LIMIT_MB);
    expect(finishedLimit('0', 500)).toBe(MIN_LIMIT_MB);
    expect(finishedLimit('-300', 500)).toBe(MIN_LIMIT_MB);
  });

  it('rounds a fraction', () => {
    expect(finishedLimit('640.6', 500)).toBe(641);
  });

  it('keeps the current limit for text that is no number', () => {
    expect(finishedLimit('abc', 700)).toBe(700);
  });
});
