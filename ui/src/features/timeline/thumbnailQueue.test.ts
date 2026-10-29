import { describe, expect, it } from 'vitest';

import {
  rankOf,
  THUMBNAIL_DELAY_MS,
  ThumbnailQueue,
  type Clock,
  type Rank,
} from './thumbnailQueue';

const view = { top: 100, bottom: 900, left: 0 };

/** A tile at `top`/`left` (tiles are 150 px high). */
function at(top: number, left = 0) {
  return { top, bottom: top + 150, left };
}

describe('rankOf', () => {
  it('puts visible tiles first, top to bottom and left to right', () => {
    const order = [at(500, 200), at(800), at(-100), at(500, 0), at(1000), at(120)]
      .map((tile) => rankOf(tile, view))
      .map((rank, i) => ({ rank, i }))
      .toSorted((a, b) => a.rank[0] - b.rank[0] || a.rank[1] - b.rank[1] || a.rank[2] - b.rank[2])
      .map(({ i }) => i);

    // Visible: 120, 500 (left 0), 500 (left 200), 800; then off screen by distance:
    // the tile at -100 ends 50 px above the view, the one at 1000 starts 100 px below.
    expect(order).toEqual([5, 3, 0, 1, 2, 4]);
  });

  it('counts a tile cut by the edge of the view as visible', () => {
    expect(rankOf(at(850), view)[0]).toBe(0);
    expect(rankOf(at(0), view)[0]).toBe(0);
  });
});

/** A clock that only moves when the test says so. */
function fakeClock() {
  let time = 0;
  let timers: { at: number; run: () => void }[] = [];
  const clock: Clock = {
    now: () => time,
    after: (delayMs, run) => timers.push({ at: time + delayMs, run }),
  };
  const advance = (ms: number) => {
    time += ms;
    const due = timers.filter((timer) => timer.at <= time);
    timers = timers.filter((timer) => timer.at > time);
    due.forEach((timer) => timer.run());
  };
  return { clock, advance };
}

/** A queue of tiles with fixed ranks, recording which ones started. */
function setUp(slots: number) {
  const { clock, advance } = fakeClock();
  const queue = new ThumbnailQueue(slots, clock);
  const started: string[] = [];
  const request = (name: string, rank: Rank | (() => Rank)) =>
    queue.request(typeof rank === 'function' ? rank : () => rank, () => started.push(name));
  return { started, request, advance };
}

describe('ThumbnailQueue', () => {
  it('lets a tile load only after it waited for the delay', () => {
    const { started, request, advance } = setUp(2);
    request('a', [0, 1, 0]);

    advance(THUMBNAIL_DELAY_MS - 1);
    expect(started).toEqual([]);

    advance(1);
    expect(started).toEqual(['a']);
  });

  it('never starts a tile that left before the delay, e.g. one flying past', () => {
    const { started, request, advance } = setUp(2);

    request('flying past', [0, 1, 0]).release();
    advance(THUMBNAIL_DELAY_MS);

    expect(started).toEqual([]);
  });

  it('ranks the tiles rendered together, whatever order they asked in', () => {
    const { started, request, advance } = setUp(2);
    // Rows rendered ahead above the screen come first in the page.
    request('above', [1, 20, 0]);
    request('top', [0, 100, 0]);
    request('below it', [0, 300, 0]);

    advance(THUMBNAIL_DELAY_MS);

    expect(started).toEqual(['top', 'below it']);
  });

  it('gives a freed slot to the best ranked waiting tile', () => {
    const { started, request, advance } = setUp(1);
    const first = request('first', [0, 0, 0]);
    request('ahead', [1, 50, 0]);
    request('lower', [0, 300, 0]);
    request('higher', [0, 200, 0]);
    advance(THUMBNAIL_DELAY_MS);

    first.release();

    expect(started).toEqual(['first', 'higher']);
  });

  it('loads tiles rendered ahead off screen once no visible tile waits', () => {
    const { started, request, advance } = setUp(1);
    const visible = request('visible', [0, 0, 0]);
    request('far', [1, 400, 0]);
    request('near', [1, 20, 0]);
    advance(THUMBNAIL_DELAY_MS);

    visible.release();

    expect(started).toEqual(['visible', 'near']);
  });

  it('never starts a tile that left before its turn', () => {
    const { started, request, advance } = setUp(1);
    const first = request('first', [0, 0, 0]);
    const gone = request('gone', [0, 1, 0]);
    request('stays', [0, 2, 0]);
    advance(THUMBNAIL_DELAY_MS);

    gone.release();
    first.release();

    expect(started).toEqual(['first', 'stays']);
  });

  it('asks for ranks when it hands out a turn, so tiles can move meanwhile', () => {
    const { started, request, advance } = setUp(1);
    const first = request('first', [0, 0, 0]);
    let scrolledInto: Rank = [1, 500, 0];
    request('scrolled into view', () => scrolledInto);
    request('near', [1, 10, 0]);
    advance(THUMBNAIL_DELAY_MS);

    scrolledInto = [0, 300, 0];
    first.release();

    expect(started).toEqual(['first', 'scrolled into view']);
  });

  it('waits for a tile that is not ready yet when a slot frees up', () => {
    const { started, request, advance } = setUp(1);
    const first = request('first', [0, 0, 0]);
    advance(THUMBNAIL_DELAY_MS);
    request('newcomer', [0, 1, 0]);

    first.release();
    expect(started).toEqual(['first']);

    advance(THUMBNAIL_DELAY_MS);
    expect(started).toEqual(['first', 'newcomer']);
  });

  it('frees a slot only once, however often a ticket is released', () => {
    const { started, request, advance } = setUp(1);
    const first = request('first', [0, 0, 0]);
    request('second', [0, 1, 0]);
    request('third', [0, 2, 0]);
    advance(THUMBNAIL_DELAY_MS);

    first.release();
    first.release();

    expect(started).toEqual(['first', 'second']);
  });
});
