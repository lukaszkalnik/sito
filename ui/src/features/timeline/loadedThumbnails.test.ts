import { beforeEach, describe, expect, it } from 'vitest';

import { forgetLoaded, rememberLoaded, shapeFrom, shapeOf } from './loadedThumbnails';

describe('loadedThumbnails', () => {
  beforeEach(() => {
    forgetLoaded();
  });

  it('knows nothing about a thumbnail not shown yet', () => {
    expect(shapeOf('thumb://localhost/1')).toBeUndefined();
  });

  it('remembers the shape of a shown thumbnail', () => {
    rememberLoaded('thumb://localhost/1', 'portrait');
    rememberLoaded('thumb://localhost/2', 'landscape');

    expect(shapeOf('thumb://localhost/1')).toBe('portrait');
    expect(shapeOf('thumb://localhost/2')).toBe('landscape');
  });

  it.each([
    [1360, 2008, 'portrait'],
    [2008, 1360, 'landscape'],
    [500, 500, 'landscape'],
  ] as const)('calls a %i × %i image %s', (width, height, shape) => {
    expect(shapeFrom(width, height)).toBe(shape);
  });
});
