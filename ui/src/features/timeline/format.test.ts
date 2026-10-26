import { describe, expect, it } from 'vitest';

import { formatDay, timeOf } from './format';

describe('formatDay', () => {
  it.each([
    ['2024-06-01', 'sobota, 1 czerwca 2024'],
    ['2026-10-26', 'poniedziałek, 26 października 2026'],
    ['2024-12-31', 'wtorek, 31 grudnia 2024'],
  ])('formats %s in Polish', (date, expected) => {
    expect(formatDay(date)).toBe(expected);
  });
});

describe('timeOf', () => {
  it('returns the time of day', () => {
    expect(timeOf('2024-06-01T12:00:05')).toBe('12:00:05');
  });
});
