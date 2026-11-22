import { describe, expect, it } from 'vitest';

import { initialMoveState, moveReducer, type MoveState, type MoveTarget } from './moveReducer';

const chosen: MoveTarget = {
  kind: 'chosen',
  folder: 'D:\\Cache',
  checked: { target: 'D:\\Cache\\Sito-cache', network: false, size: 1000 },
};

function run(...actions: Parameters<typeof moveReducer>[1][]): MoveState {
  return actions.reduce(moveReducer, initialMoveState);
}

describe('moveReducer', () => {
  it('goes from choosing over confirming and moving back to idle', () => {
    expect(run({ type: 'choose' })).toEqual({ kind: 'choosing' });
    expect(run({ type: 'choose' }, { type: 'chosen', target: chosen })).toEqual({
      kind: 'confirming',
      target: chosen,
    });
    const moving = run(
      { type: 'choose' },
      { type: 'chosen', target: chosen },
      { type: 'start' },
      { type: 'progress', progress: { done: 3, total: 5 } },
    );
    expect(moving).toEqual({ kind: 'moving', progress: { done: 3, total: 5 } });
    expect(moveReducer(moving, { type: 'finished' })).toEqual(initialMoveState);
  });

  it('starts moving without progress', () => {
    expect(run({ type: 'chosen', target: { kind: 'default' } }, { type: 'start' })).toEqual({
      kind: 'moving',
      progress: null,
    });
  });

  it('goes back to idle when no folder is picked', () => {
    expect(run({ type: 'choose' }, { type: 'notChosen' })).toEqual(initialMoveState);
  });

  it('keeps a failure until the next attempt', () => {
    const failed = run({ type: 'choose' }, { type: 'failed', code: 'insideOpenFolder' });
    expect(failed).toEqual({ kind: 'failed', code: 'insideOpenFolder' });
    expect(moveReducer(failed, { type: 'choose' })).toEqual({ kind: 'choosing' });
    expect(moveReducer(failed, { type: 'dismiss' })).toEqual(initialMoveState);
  });

  it('cannot be dismissed or started again while moving', () => {
    const moving = run({ type: 'chosen', target: chosen }, { type: 'start' });
    expect(moveReducer(moving, { type: 'dismiss' })).toBe(moving);
    expect(moveReducer(moving, { type: 'choose' })).toBe(moving);
    expect(moveReducer(moving, { type: 'chosen', target: chosen })).toBe(moving);
  });

  it('ignores progress and starts that do not fit', () => {
    expect(run({ type: 'progress', progress: { done: 1, total: 2 } })).toEqual(initialMoveState);
    expect(run({ type: 'start' })).toEqual(initialMoveState);
    expect(run({ type: 'notChosen' })).toEqual(initialMoveState);
  });

  it('a dismissed confirmation moves nothing', () => {
    expect(run({ type: 'chosen', target: chosen }, { type: 'dismiss' }, { type: 'start' })).toEqual(
      initialMoveState,
    );
  });
});
