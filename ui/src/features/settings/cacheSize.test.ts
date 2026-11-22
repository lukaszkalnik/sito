import { describe, expect, it } from 'vitest';

import { formatsFor } from '../../i18n/formats';
import { cacheSizeText } from './cacheSize';

const MB = 1024 * 1024;

describe('cacheSizeText', () => {
  it('shows whole MB from 1 MB', () => {
    expect(cacheSizeText(412 * MB + 300_000, formatsFor('pl'))).toEqual({
      value: '412',
      unit: 'mb',
    });
    expect(cacheSizeText(MB, formatsFor('pl'))).toEqual({ value: '1', unit: 'mb' });
  });

  it('shows whole kB below 1 MB, also nothing', () => {
    expect(cacheSizeText(18 * 1024, formatsFor('pl'))).toEqual({ value: '18', unit: 'kb' });
    expect(cacheSizeText(0, formatsFor('pl'))).toEqual({ value: '0', unit: 'kb' });
  });

  it('groups large numbers as the locale does', () => {
    expect(cacheSizeText(15_360 * MB, formatsFor('pl')).value).toBe('15\u00a0360');
    expect(cacheSizeText(15_360 * MB, formatsFor('en')).value).toBe('15,360');
    expect(cacheSizeText(15_360 * MB, formatsFor('de')).value).toBe('15.360');
  });
});
