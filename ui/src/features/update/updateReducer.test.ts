import { describe, expect, it } from 'vitest';

import type { UpdateInfo } from './api';
import {
  initialUpdateState,
  isShown,
  updateReducer,
  type UpdateEvent,
  type UpdateState,
} from './updateReducer';

const info: UpdateInfo = { version: '0.15.0', notes: '### New', install: 'inApp' };

function run(...events: UpdateEvent[]): UpdateState {
  return events.reduce(updateReducer, initialUpdateState);
}

describe('updateReducer', () => {
  it('offers an update found at the start', () => {
    const state = run({ type: 'checkStarted', trigger: 'start' }, { type: 'found', info });

    expect(state).toEqual({ kind: 'available', info, failure: null });
    expect(isShown(state)).toBe(true);
  });

  it('stays quiet at the start while checking, without an update and when offline', () => {
    const checking = run({ type: 'checkStarted', trigger: 'start' });
    expect(isShown(checking)).toBe(false);

    expect(updateReducer(checking, { type: 'notFound', currentVersion: '0.14.0' })).toEqual(
      initialUpdateState,
    );
    expect(updateReducer(checking, { type: 'checkFailed', code: 'network' })).toEqual(
      initialUpdateState,
    );
  });

  it('answers a check from the menu in every case', () => {
    const checking = run({ type: 'checkStarted', trigger: 'menu' });
    expect(isShown(checking)).toBe(true);

    expect(updateReducer(checking, { type: 'notFound', currentVersion: '0.14.0' })).toEqual({
      kind: 'upToDate',
      version: '0.14.0',
    });
    expect(updateReducer(checking, { type: 'checkFailed', code: 'network' })).toEqual({
      kind: 'failed',
      code: 'network',
    });
  });

  it('goes from downloading with progress to installing', () => {
    const downloading = run(
      { type: 'checkStarted', trigger: 'start' },
      { type: 'found', info },
      { type: 'installStarted' },
    );
    expect(downloading).toEqual({ kind: 'downloading', info, fraction: null });

    const halfway = updateReducer(downloading, {
      type: 'progress',
      progress: { kind: 'downloading', downloaded: 50, total: 200 },
    });
    expect(halfway).toEqual({ kind: 'downloading', info, fraction: 0.25 });

    expect(updateReducer(halfway, { type: 'progress', progress: { kind: 'installing' } })).toEqual({
      kind: 'installing',
      info,
    });
  });

  it('keeps the progress unknown when the size is unknown', () => {
    const state = run(
      { type: 'checkStarted', trigger: 'start' },
      { type: 'found', info },
      { type: 'installStarted' },
      { type: 'progress', progress: { kind: 'downloading', downloaded: 50, total: null } },
    );

    expect(state).toEqual({ kind: 'downloading', info, fraction: null });
  });

  it('offers the update again with the reason when installing fails', () => {
    const state = run(
      { type: 'checkStarted', trigger: 'start' },
      { type: 'found', info },
      { type: 'installStarted' },
      { type: 'installFailed', code: 'badSignature' },
    );

    expect(state).toEqual({
      kind: 'available',
      info,
      failure: { action: 'install', code: 'badSignature' },
    });
  });

  it('keeps the offer with the reason when the release page does not open', () => {
    const failed = run(
      { type: 'checkStarted', trigger: 'start' },
      { type: 'found', info: { ...info, install: 'manual' } },
      { type: 'openPageFailed', code: 'openFailed' },
    );
    expect(failed).toEqual({
      kind: 'available',
      info: { ...info, install: 'manual' },
      failure: { action: 'openPage', code: 'openFailed' },
    });

    const checkedAgain = [
      { type: 'checkStarted', trigger: 'menu' },
      { type: 'found', info },
    ] as const satisfies readonly UpdateEvent[];
    expect(checkedAgain.reduce(updateReducer, failed)).toEqual({
      kind: 'available',
      info,
      failure: null,
    });
  });

  it('ignores a release page failure without an offer', () => {
    expect(run({ type: 'openPageFailed', code: 'openFailed' })).toEqual(initialUpdateState);
  });

  it('is not interrupted by a check or a dismissal while installing', () => {
    const downloading = run(
      { type: 'checkStarted', trigger: 'start' },
      { type: 'found', info },
      { type: 'installStarted' },
    );

    expect(updateReducer(downloading, { type: 'checkStarted', trigger: 'menu' })).toBe(downloading);
    expect(updateReducer(downloading, { type: 'dismissed' })).toBe(downloading);
  });

  it('hides the notice when dismissed', () => {
    const state = run(
      { type: 'checkStarted', trigger: 'start' },
      { type: 'found', info },
      { type: 'dismissed' },
    );

    expect(state).toEqual(initialUpdateState);
    expect(isShown(state)).toBe(false);
  });

  it('ignores answers that arrive when no check is running', () => {
    expect(run({ type: 'found', info })).toEqual(initialUpdateState);
    expect(run({ type: 'notFound', currentVersion: '0.14.0' })).toEqual(initialUpdateState);
  });
});
