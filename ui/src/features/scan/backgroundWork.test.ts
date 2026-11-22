import { describe, expect, it } from 'vitest';

import type { ScanSummary } from './api';
import { backgroundWork } from './backgroundWork';
import type { ScanState } from './scanReducer';

const summary: ScanSummary = {
  scanId: 1,
  photosFound: 3,
  unreadableEntries: 0,
  withoutDate: 0,
  refreshing: false,
};

function done(
  refreshing: boolean,
  completing: { done: number; total: number } | null,
): Extract<ScanState, { status: 'done' }> {
  return {
    status: 'done',
    path: '/photos',
    summary: { ...summary, refreshing },
    completing,
    missing: 'none',
  };
}

describe('backgroundWork', () => {
  it('is nothing without an open folder or when it is settled', () => {
    expect(backgroundWork({ status: 'idle' })).toBe('none');
    expect(backgroundWork({ status: 'error', path: '/x', code: 'notFound' })).toBe('none');
    expect(backgroundWork(done(false, null))).toBe('none');
  });

  it('is a scan while scanning or refreshing', () => {
    expect(
      backgroundWork({
        status: 'scanning',
        path: '/photos',
        progress: { phase: 'searching', found: 1 },
      }),
    ).toBe('scanning');
    expect(backgroundWork(done(true, null))).toBe('scanning');
    expect(backgroundWork(done(true, { done: 1, total: 9 }))).toBe('scanning');
  });

  it('is the EXIF pass while it reads', () => {
    expect(backgroundWork(done(false, { done: 1, total: 9 }))).toBe('exifPass');
  });

  it('is the shown folder’s while another folder is picked', () => {
    expect(backgroundWork({ status: 'picking', previous: done(true, null) })).toBe('scanning');
    expect(backgroundWork({ status: 'picking', previous: { status: 'idle' } })).toBe('none');
  });
});
