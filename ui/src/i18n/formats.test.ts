import { describe, expect, it } from 'vitest';

import { formatsFor } from './formats';

// Intl separates groups and percent signs with (narrow) no-break spaces.
const spaces = (text: string) => text.replace(/[  ]/gu, ' ');

const JUNE_1 = new Date(2024, 5, 1);

describe('formatsFor', () => {
  it.each([
    ['pl-PL', 'sobota, 1 czerwca 2024', '50%', '12 346', '5472', '7,5'],
    ['en-GB', 'Saturday, 1 June 2024', '50%', '12,346', '5,472', '7.5'],
    ['en-US', 'Saturday, June 1, 2024', '50%', '12,346', '5,472', '7.5'],
    ['de-DE', 'Samstag, 1. Juni 2024', '50 %', '12.346', '5.472', '7,5'],
  ])('formats in %s', (locale, day, percent, large, small, decimal) => {
    const formats = formatsFor(locale);
    expect(formats.locale).toBe(locale);
    expect(formats.longDay(JUNE_1)).toBe(day);
    expect(spaces(formats.percent(0.5))).toBe(percent);
    expect(spaces(formats.whole(12_345.6))).toBe(large);
    expect(spaces(formats.whole(5472))).toBe(small);
    expect(formats.oneDecimal(7.47)).toBe(decimal);
  });

  it.each([
    ['pl', 'pl-PL'],
    ['en', 'en-GB'],
    ['de', 'de-DE'],
  ])('gives the language %s its default locale %s', (language, locale) => {
    expect(formatsFor(language).locale).toBe(locale);
  });

  it('builds the formats of a locale once', () => {
    expect(formatsFor('de-AT')).toBe(formatsFor('de-AT'));
    expect(formatsFor('en')).toBe(formatsFor('en-GB'));
  });

  it('rounds percentages to whole numbers', () => {
    expect(formatsFor('pl-PL').percent(0.666)).toBe('67%');
  });
});
