import { describe, expect, it } from 'vitest';

import { formatsFor } from '../../i18n/formats';
import { fileSize } from './photoDetails';

const polish = formatsFor('pl-PL');

describe('fileSize', () => {
  it('gives megabytes with one decimal and a comma', () => {
    expect(fileSize(7_835_038, polish)).toEqual({ value: '7,5', unit: 'mb' });
    expect(fileSize(1024 * 1024, polish)).toEqual({ value: '1,0', unit: 'mb' });
  });

  it('gives whole kilobytes below a megabyte, at least 1', () => {
    expect(fileSize(831_488, polish)).toEqual({ value: '812', unit: 'kb' });
    expect(fileSize(10, polish)).toEqual({ value: '1', unit: 'kb' });
  });

  it('uses the decimal point of the locale', () => {
    expect(fileSize(7_835_038, formatsFor('en-GB'))).toEqual({ value: '7.5', unit: 'mb' });
  });
});
