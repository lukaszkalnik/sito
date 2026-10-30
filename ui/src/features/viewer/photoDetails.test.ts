import { describe, expect, it } from 'vitest';

import { fileSize, formatPixels } from './photoDetails';

describe('fileSize', () => {
  it('gives megabytes with one decimal and a comma', () => {
    expect(fileSize(7_835_038)).toEqual({ value: '7,5', unit: 'mb' });
    expect(fileSize(1024 * 1024)).toEqual({ value: '1,0', unit: 'mb' });
  });

  it('gives whole kilobytes below a megabyte, at least 1', () => {
    expect(fileSize(831_488)).toEqual({ value: '812', unit: 'kb' });
    expect(fileSize(10)).toEqual({ value: '1', unit: 'kb' });
  });
});

describe('formatPixels', () => {
  it('groups large numbers the Polish way', () => {
    expect(formatPixels(5472)).toBe('5472');
    expect(formatPixels(12_000).replace(/\s/u, ' ')).toBe('12 000');
  });
});
