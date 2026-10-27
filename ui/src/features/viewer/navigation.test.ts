import { describe, expect, it } from 'vitest';

import type { Series } from '../../api/scan';
import { navigationOrder, neighbours, seriesOf } from './navigation';

// Ten photos; series at positions 1–2 and 5–7.
const series: Series[] = [
  { start: 1, len: 2 },
  { start: 5, len: 3 },
];

describe('navigationOrder', () => {
  it('walks every photo in the all view', () => {
    expect(navigationOrder(4, series, 'all')).toEqual([0, 1, 2, 3]);
  });

  it('walks only series photos in the series view', () => {
    expect(navigationOrder(10, series, 'series')).toEqual([1, 2, 5, 6, 7]);
  });
});

describe('neighbours', () => {
  const order = navigationOrder(10, series, 'series');

  it('crosses from one series into the next', () => {
    expect(neighbours(order, 2)).toEqual({ previous: 1, next: 5 });
    expect(neighbours(order, 5)).toEqual({ previous: 2, next: 6 });
  });

  it('stops at the ends', () => {
    expect(neighbours(order, 1)).toEqual({ previous: null, next: 2 });
    expect(neighbours(order, 7)).toEqual({ previous: 6, next: null });
  });

  it('finds the surrounding photos of a photo outside the order', () => {
    expect(neighbours(order, 3)).toEqual({ previous: 2, next: 5 });
    expect(neighbours(order, 0)).toEqual({ previous: null, next: 1 });
    expect(neighbours(order, 9)).toEqual({ previous: 7, next: null });
  });

  it('has no neighbours in an empty order', () => {
    expect(neighbours([], 3)).toEqual({ previous: null, next: null });
  });
});

describe('seriesOf', () => {
  it('finds the series of a photo', () => {
    expect(seriesOf(series, 6)).toEqual({ start: 5, len: 3 });
    expect(seriesOf(series, 1)).toEqual({ start: 1, len: 2 });
  });

  it('is undefined for single photos', () => {
    expect(seriesOf(series, 0)).toBeUndefined();
    expect(seriesOf(series, 3)).toBeUndefined();
    expect(seriesOf(series, 8)).toBeUndefined();
  });
});
