import { afterEach, describe, expect, it } from 'vitest';

import { toLanguageInfo } from './api';
import i18n from './index';
import { applyLanguage } from './language';

// Intl groups thousands with (narrow) no-break spaces.
const spaces = (text: string) => text.replace(/[  ]/gu, ' ');

afterEach(async () => {
  await applyLanguage({ language: 'pl', locale: 'pl-PL', choice: 'system' });
});

describe('Polish plurals', () => {
  it.each([
    [1, '1 zdjęcie'],
    [2, '2 zdjęcia'],
    [4, '4 zdjęcia'],
    [5, '5 zdjęć'],
    [12, '12 zdjęć'],
    [22, '22 zdjęcia'],
    [25, '25 zdjęć'],
  ])('formats %i photos', (count, expected) => {
    expect(i18n.t('scan.photoCount', { count })).toBe(expected);
  });

  it('formats numbers with the pl-PL locale', () => {
    expect(spaces(i18n.t('scan.photoCount', { count: 12345 }))).toBe('12 345 zdjęć');
  });
});

describe.each([
  ['en-GB', 'en', '0 photos', '1 photo', '2 photos', '5 photos', '12,345 photos'],
  ['en-US', 'en', '0 photos', '1 photo', '2 photos', '5 photos', '12,345 photos'],
  ['de-DE', 'de', '0 Fotos', '1 Foto', '2 Fotos', '5 Fotos', '12.345 Fotos'],
  ['de-AT', 'de', '0 Fotos', '1 Foto', '2 Fotos', '5 Fotos', '12 345 Fotos'],
] as const)('in %s', (locale, language, zero, one, two, five, many) => {
  it('picks the plural form and the number format of the locale', async () => {
    await applyLanguage({ language, locale, choice: language });
    const count = (n: number) => spaces(i18n.t('scan.photoCount', { count: n }));
    expect([count(0), count(1), count(2), count(5), count(12_345)]).toEqual([
      zero,
      one,
      two,
      five,
      many,
    ]);
  });

  it('sets the language of the page', async () => {
    await applyLanguage({ language, locale, choice: 'system' });
    expect(document.documentElement.lang).toBe(locale);
  });
});

describe('switching the language', () => {
  it('changes the texts at once', async () => {
    await applyLanguage({ language: 'de', locale: 'de-DE', choice: 'de' });
    expect(i18n.t('reject.button')).toBe('Ablehnen');
    await applyLanguage({ language: 'en', locale: 'en-GB', choice: 'en' });
    expect(i18n.t('reject.button')).toBe('Reject');
    await applyLanguage({ language: 'pl', locale: 'pl-PL', choice: 'pl' });
    expect(i18n.t('reject.button')).toBe('Odrzuć');
  });
});

describe('toLanguageInfo', () => {
  it('accepts what the backend sends', () => {
    expect(toLanguageInfo({ language: 'de', locale: 'de-AT', choice: 'system' })).toEqual({
      language: 'de',
      locale: 'de-AT',
      choice: 'system',
    });
  });

  it.each([null, 'pl', {}, { language: 'fr', locale: 'fr-FR', choice: 'system' }])(
    'refuses %j',
    (raw) => {
      expect(toLanguageInfo(raw)).toBeNull();
    },
  );

  it('refuses an unknown choice', () => {
    expect(toLanguageInfo({ language: 'pl', locale: 'pl-PL', choice: 'fr' })).toBeNull();
  });

  it.each(['en US', 'pl-PL', '', 42])(
    'uses the language alone for the locale %j of en',
    (locale) => {
      expect(toLanguageInfo({ language: 'en', locale, choice: 'en' })?.locale).toBe('en');
    },
  );
});
