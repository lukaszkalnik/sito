import { describe, expect, it } from 'vitest';

import i18n from './index';

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
    // pl-PL groups thousands with a non-breaking space.
    expect(i18n.t('scan.photoCount', { count: 12345 })).toBe('12 345 zdjęć');
  });
});
