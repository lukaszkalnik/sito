import { describe, expect, it } from 'vitest';

import { initialScanState, scanReducer, type ScanState } from './scanReducer';

const summary = { photosFound: 3, unreadableEntries: 0, withoutExifDate: 0 };
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
    expect(state).toEqual({ status: 'done', path: 'C:\\Photos', summary });
  });

  it('restores the previous result when picking is cancelled', () => {
    const done: ScanState = { status: 'done', path: 'C:\\Photos', summary };

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
});
