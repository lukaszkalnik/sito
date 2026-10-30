import { describe, expect, it } from 'vitest';

import {
  followedId,
  followReducer,
  lastShownId,
  NOT_FOLLOWING,
  type FollowAction,
} from './followViewer';

const shown = (id: number, scanId = 1): FollowAction => ({ kind: 'shown', scanId, id });
const run = (...actions: FollowAction[]) => actions.reduce(followReducer, NOT_FOLLOWING);

describe('followReducer', () => {
  it('follows the latest photo the viewer shows', () => {
    const state = run(shown(3), shown(4));

    expect(followedId(state, 1)).toBe(4);
    expect(lastShownId(state, 1)).toBeNull();
  });

  it('ignores photos of another scan', () => {
    const state = run(shown(3, 2));

    expect(followedId(state, 1)).toBeNull();
    expect(followedId(state, 2)).toBe(3);
  });

  it('hands the last photo over to be revealed when the viewer closes', () => {
    const state = run(shown(3), shown(4), { kind: 'closed' });

    expect(followedId(state, 1)).toBeNull();
    expect(lastShownId(state, 1)).toBe(4);
    expect(lastShownId(state, 2)).toBeNull();
  });

  it('keeps the last photo to reveal when the close is reported twice', () => {
    const state = run(shown(4), { kind: 'closed' }, { kind: 'closed' });

    expect(lastShownId(state, 1)).toBe(4);
  });

  it('forgets the last photo once it was revealed', () => {
    const state = run(shown(4), { kind: 'closed' }, { kind: 'revealed' });

    expect(state).toEqual(NOT_FOLLOWING);
  });

  it('keeps following when a reveal ends while the viewer is open', () => {
    const state = run(shown(4), { kind: 'revealed' });

    expect(followedId(state, 1)).toBe(4);
  });

  it('has nothing to reveal when the viewer closes before showing a photo', () => {
    expect(run({ kind: 'closed' })).toEqual(NOT_FOLLOWING);
  });

  it('follows again when the viewer reopens before the reveal', () => {
    const state = run(shown(4), { kind: 'closed' }, shown(7));

    expect(followedId(state, 1)).toBe(7);
    expect(lastShownId(state, 1)).toBeNull();
  });
});
