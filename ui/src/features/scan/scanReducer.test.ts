import { describe, expect, it } from 'vitest';

import { initialScanState, scanReducer, type ScanState } from './scanReducer';

const summary = {
  scanId: 1,
  photosFound: 3,
  unreadableEntries: 0,
  withoutDate: 0,
  refreshing: false,
};
const scanning: ScanState = {
  status: 'scanning',
  path: 'C:\\Photos',
  progress: { phase: 'searching', found: 0 },
};

describe('scanReducer', () => {
  it('goes from idle through picking and scanning to done', () => {
    let state = scanReducer(initialScanState, { type: 'pickStarted' });
    expect(state.status).toBe('picking');

    state = scanReducer(state, { type: 'scanStarted', path: 'C:\\Photos' });
    state = scanReducer(state, {
      type: 'progress',
      progress: { phase: 'reading', done: 2, total: 5 },
    });
    expect(state).toEqual({
      status: 'scanning',
      path: 'C:\\Photos',
      progress: { phase: 'reading', done: 2, total: 5 },
    });

    state = scanReducer(state, { type: 'scanFinished', summary });
    expect(state).toEqual({ status: 'done', path: 'C:\\Photos', summary, completing: null });
  });

  it('restores the previous result when picking is cancelled', () => {
    const done: ScanState = { status: 'done', path: 'C:\\Photos', summary, completing: null };

    const picking = scanReducer(done, { type: 'pickStarted' });
    const restored = scanReducer(picking, { type: 'pickCancelled' });

    expect(restored).toBe(done);
  });

  it('returns to idle when the first pick is cancelled', () => {
    const picking = scanReducer(initialScanState, { type: 'pickStarted' });

    expect(scanReducer(picking, { type: 'pickCancelled' })).toEqual({ status: 'idle' });
  });

  it('records the error code when a scan fails', () => {
    expect(scanReducer(scanning, { type: 'scanFailed', code: 'notFound' })).toEqual({
      status: 'error',
      path: 'C:\\Photos',
      code: 'notFound',
    });
  });

  it('ignores progress after the scan has finished', () => {
    const done = scanReducer(scanning, { type: 'scanFinished', summary });

    expect(
      scanReducer(done, { type: 'progress', progress: { phase: 'searching', found: 1 } }),
    ).toBe(done);
  });

  it('does not start picking while scanning', () => {
    expect(scanReducer(scanning, { type: 'pickStarted' })).toBe(scanning);
  });

  describe('a background refresh', () => {
    const cached = { ...summary, refreshing: true };
    const fresh = { ...summary, photosFound: 4, refreshing: false };
    const done: ScanState = {
      status: 'done',
      path: 'C:\\Photos',
      summary: cached,
      completing: null,
    };

    it('updates the summary of the folder shown', () => {
      expect(scanReducer(done, { type: 'refreshed', summary: fresh })).toEqual({
        ...done,
        summary: fresh,
      });
    });

    it('also while another folder is being picked', () => {
      const picking = scanReducer(done, { type: 'pickStarted' });

      expect(scanReducer(picking, { type: 'refreshed', summary: fresh })).toEqual({
        status: 'picking',
        previous: { ...done, summary: fresh },
      });
    });

    it('is ignored for a scan that is no longer shown', () => {
      const other = { ...fresh, scanId: 99 };

      expect(scanReducer(done, { type: 'refreshed', summary: other })).toBe(done);
      expect(scanReducer(scanning, { type: 'refreshed', summary: fresh })).toBe(scanning);
    });
  });

  describe('reading EXIF in the background', () => {
    const done: ScanState = { status: 'done', path: 'C:\\Photos', summary, completing: null };
    const progress = (done: number, total: number) => ({
      type: 'completing' as const,
      completing: { scanId: 1, done, total },
    });

    it('records its progress until it is complete', () => {
      const running = scanReducer(done, progress(200, 500));
      expect(running).toEqual({ ...done, completing: { done: 200, total: 500 } });

      expect(scanReducer(running, progress(500, 500))).toEqual(done);
    });

    it('also while another folder is being picked', () => {
      const picking = scanReducer(done, { type: 'pickStarted' });

      expect(scanReducer(picking, progress(200, 500))).toEqual({
        status: 'picking',
        previous: { ...done, completing: { done: 200, total: 500 } },
      });
    });

    it('is ignored for a scan that is no longer shown', () => {
      const other = { type: 'completing' as const, completing: { scanId: 99, done: 1, total: 2 } };

      expect(scanReducer(done, other)).toBe(done);
      expect(scanReducer(scanning, progress(1, 2))).toBe(scanning);
    });
  });
});
