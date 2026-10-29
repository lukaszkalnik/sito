/**
 * Which tile loads its thumbnail next. The backend renders only a couple at
 * a time and an uncached one takes up to a second, so tiles take turns: at
 * most {@link THUMBNAIL_SLOTS} at once, the visible ones top to bottom
 * first, then (while nothing visible waits) those rendered ahead off screen,
 * nearest first. Only the tiles the grid renders (the screen plus a few rows
 * around it) ever queue, so loading ahead stops there. A tile gets a turn
 * only after waiting {@link THUMBNAIL_DELAY_MS}, so tiles that just fly past
 * during fast scrolling never load, and a tile that goes away before its
 * turn never loads.
 */

/** Loads at once: one more than the backend's render workers, so neither idles. */
export const THUMBNAIL_SLOTS = 3;

/** How long a tile must wait (stay rendered) before it may load its thumbnail. */
export const THUMBNAIL_DELAY_MS = 150;

/** A rectangle on screen, as `getBoundingClientRect` gives it. */
export interface ScreenRect {
  readonly top: number;
  readonly bottom: number;
  readonly left: number;
}

/** Sort key of a waiting tile: lower goes first. */
export type Rank = readonly [group: number, distance: number, left: number];

/** The rank of a tile at `tile` in a scroll area showing `view`. */
export function rankOf(tile: ScreenRect, view: ScreenRect): Rank {
  if (tile.bottom > view.top && tile.top < view.bottom) return [0, tile.top, tile.left];
  const distance = tile.top >= view.bottom ? tile.top - view.bottom : view.top - tile.bottom;
  return [1, distance, tile.left];
}

function isVisible(rank: Rank): boolean {
  return rank[0] === 0;
}

function compareRanks(a: Rank, b: Rank): number {
  return a[0] - b[0] || a[1] - b[1] || a[2] - b[2];
}

/** A tile's place in the queue. */
export interface Ticket {
  /**
   * Leaves the queue: a waiting tile never gets its turn, a loading one
   * frees its slot. Safe to call more than once.
   */
  release(): void;
}

/** Time in milliseconds, and running a function after a delay. */
export interface Clock {
  now(): number;
  after(delayMs: number, run: () => void): void;
}

const realClock: Clock = {
  now: () => Date.now(),
  after: (delayMs, run) => {
    setTimeout(run, delayMs);
  },
};

interface Entry {
  readonly rank: () => Rank;
  readonly start: () => void;
  /** When the tile joined the queue. */
  readonly since: number;
  state: 'waiting' | 'loading' | 'released';
}

/** Hands out turns to load thumbnails, {@link THUMBNAIL_SLOTS} at a time by default. */
export class ThumbnailQueue {
  private readonly slots: number;
  private readonly clock: Clock;
  private readonly waiting = new Set<Entry>();
  private loading = 0;
  /** Times a hand-out is scheduled for. */
  private readonly wakeUps = new Set<number>();

  constructor(slots = THUMBNAIL_SLOTS, clock: Clock = realClock) {
    this.slots = slots;
    this.clock = clock;
  }

  /**
   * Queues a tile. `rank` is asked each time a turn is handed out, so it
   * follows the tile while it scrolls; `start` is called when its turn comes.
   */
  request(rank: () => Rank, start: () => void): Ticket {
    const entry: Entry = { rank, start, since: this.clock.now(), state: 'waiting' };
    this.waiting.add(entry);
    this.wakeUp(entry.since + THUMBNAIL_DELAY_MS);
    return { release: () => this.release(entry) };
  }

  private release(entry: Entry): void {
    const was = entry.state;
    entry.state = 'released';
    if (was === 'waiting') this.waiting.delete(entry);
    if (was !== 'loading') return;
    this.loading--;
    this.handOut();
  }

  /** Hands out turns at `at`; tiles rendered together share one wake-up. */
  private wakeUp(at: number): void {
    if (this.wakeUps.has(at)) return;
    this.wakeUps.add(at);
    this.clock.after(Math.max(0, at - this.clock.now()), () => {
      this.wakeUps.delete(at);
      this.handOut();
    });
  }

  /** Gives free slots to the best ranked tiles that have waited long enough. */
  private handOut(): void {
    while (this.loading < this.slots) {
      const next = this.best();
      if (next === undefined) return;
      this.waiting.delete(next);
      next.state = 'loading';
      this.loading++;
      next.start();
    }
  }

  /**
   * The best ranked tile that has waited long enough. A tile off screen never
   * goes while a visible one is still in its delay: rows rendered in an
   * earlier frame would otherwise take the slots just before the screen's.
   */
  private best(): Entry | undefined {
    const now = this.clock.now();
    let best: { entry: Entry; rank: Rank } | undefined;
    let soonest = Infinity;
    let visibleNotReady = false;
    for (const entry of this.waiting) {
      const rank = entry.rank();
      const ready = entry.since + THUMBNAIL_DELAY_MS;
      if (ready > now) {
        soonest = Math.min(soonest, ready);
        visibleNotReady ||= isVisible(rank);
        continue;
      }
      if (best === undefined || compareRanks(rank, best.rank) < 0) best = { entry, rank };
    }
    if (best !== undefined && (isVisible(best.rank) || !visibleNotReady)) return best.entry;
    if (soonest !== Infinity) this.wakeUp(soonest);
    return undefined;
  }
}

/** The queue all timeline tiles share. */
export const thumbnailQueue = new ThumbnailQueue();
