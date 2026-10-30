import type { Virtualizer } from '@tanstack/react-virtual';
import { useEffect, useLayoutEffect, useRef, useState, type RefObject, type UIEvent } from 'react';

import { anchoredScrollTop, revealScrollTop, type TimelineRow } from './layout';
import { HIGHLIGHT_MS, type Reveal } from './reveal';

/** How long to wait for a revealed photo to appear in the rows. */
const REVEAL_WAIT_MS = 2000;
/** How long the followed photo must stay before the grid scrolls to it, so holding an arrow key in the viewer stays smooth. */
export const FOLLOW_DELAY_MS = 150;

type ScrollTopRef = RefObject<number>;

/** Tracks an element's content width; 0 until it has been measured. */
export function useElementWidth(ref: RefObject<HTMLElement | null>): number {
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const element = ref.current;
    if (!element || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(([first]) => {
      if (first) setWidth(first.contentRect.width);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref]);
  return width;
}

/** Scrolls `element` to `top` and remembers where it ended up. */
function scrollTo(element: HTMLElement, top: number, lastScrollTop: ScrollTopRef) {
  element.scrollTop = top;
  lastScrollTop.current = element.scrollTop;
}

/** Keeps the photos at the top of the view in place when the rows change, before paint. */
function useAnchoredRows(
  scrollRef: RefObject<HTMLElement | null>,
  rows: readonly TimelineRow[],
  virtualizer: Virtualizer<HTMLDivElement, Element>,
  lastScrollTop: ScrollTopRef,
) {
  const previousRows = useRef(rows);
  useLayoutEffect(() => {
    const oldRows = previousRows.current;
    previousRows.current = rows;
    virtualizer.measure();
    const element = scrollRef.current;
    if (!element || oldRows === rows) return;
    scrollTo(element, anchoredScrollTop(oldRows, rows, lastScrollTop.current), lastScrollTop);
  }, [rows, virtualizer, scrollRef, lastScrollTop]);
}

/**
 * Scrolls to `reveal` as soon as the rows show it and returns it as the
 * photo to highlight for its {@link HIGHLIGHT_MS}. The rows may still be the
 * old ones when `reveal` is set (a restored photo arrives with the next
 * reload), so it waits a moment before giving up.
 */
function useRevealScroll(
  scrollRef: RefObject<HTMLElement | null>,
  rows: readonly TimelineRow[],
  reveal: Reveal | null,
  onRevealed: (() => void) | undefined,
  lastScrollTop: ScrollTopRef,
): Reveal | null {
  const [highlighted, setHighlighted] = useState<Reveal | null>(null);
  const id = reveal?.id ?? null;
  const reason = reveal?.reason ?? null;

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (id === null || reason === null || !element) return;
    const target = revealScrollTop(rows, id, element.scrollTop, element.clientHeight);
    if (target === null) {
      const timer = setTimeout(() => onRevealed?.(), REVEAL_WAIT_MS);
      return () => clearTimeout(timer);
    }
    scrollTo(element, target, lastScrollTop);
    setHighlighted({ id, reason });
    onRevealed?.();
  }, [rows, id, reason, onRevealed, scrollRef, lastScrollTop]);

  useEffect(() => {
    if (highlighted === null) return;
    const timer = setTimeout(() => setHighlighted(null), HIGHLIGHT_MS[highlighted.reason]);
    return () => clearTimeout(timer);
  }, [highlighted]);

  return highlighted;
}

/**
 * Scrolls to `follow` once it has stayed for {@link FOLLOW_DELAY_MS}, only
 * if it is not fully visible; does nothing if the rows do not show it.
 */
function useFollowScroll(
  scrollRef: RefObject<HTMLElement | null>,
  rows: readonly TimelineRow[],
  follow: number | null,
  lastScrollTop: ScrollTopRef,
) {
  useEffect(() => {
    if (follow === null) return;
    const timer = setTimeout(() => {
      const element = scrollRef.current;
      if (!element) return;
      const target = revealScrollTop(rows, follow, element.scrollTop, element.clientHeight);
      if (target !== null && target !== element.scrollTop) {
        scrollTo(element, target, lastScrollTop);
      }
    }, FOLLOW_DELAY_MS);
    return () => clearTimeout(timer);
  }, [rows, follow, scrollRef, lastScrollTop]);
}

/** What the grid scrolls to besides the user's own scrolling. */
export interface GridScrollTargets {
  /** A photo to scroll into view and highlight once the rows show it. */
  reveal: Reveal | null;
  /** Called when `reveal` was shown (or could not be). */
  onRevealed: (() => void) | undefined;
  /** The photo the viewer shows, kept in view without highlighting it. */
  follow: number | null;
}

/**
 * Scrolling of the grid: keeps the photos at the top of the view in place
 * when the rows change (new gap, mode or width), so the grid never visibly
 * jumps; reveals and follows photos as described in {@link GridScrollTargets}.
 * Returns the highlighted photo and the grid's scroll handler.
 */
export function useGridScroll(
  scrollRef: RefObject<HTMLElement | null>,
  rows: readonly TimelineRow[],
  virtualizer: Virtualizer<HTMLDivElement, Element>,
  { reveal, onRevealed, follow }: GridScrollTargets,
) {
  // The scroll position in the old rows. Read from scroll events, because by
  // the time the effect runs, a shorter new canvas may already have made the
  // browser clamp `scrollTop`.
  const lastScrollTop = useRef(0);
  useAnchoredRows(scrollRef, rows, virtualizer, lastScrollTop);
  const highlighted = useRevealScroll(scrollRef, rows, reveal, onRevealed, lastScrollTop);
  useFollowScroll(scrollRef, rows, follow, lastScrollTop);

  const onScroll = (event: UIEvent<HTMLElement>) => {
    lastScrollTop.current = event.currentTarget.scrollTop;
  };
  return { highlighted, onScroll };
}
