import type { Virtualizer } from '@tanstack/react-virtual';
import { useEffect, useLayoutEffect, useRef, useState, type RefObject, type UIEvent } from 'react';

import { anchoredScrollTop, revealScrollTop, type TimelineRow } from './layout';

/** How long a revealed photo stays highlighted. */
const HIGHLIGHT_MS = 2600;
/** How long to wait for a revealed photo to appear in the rows. */
const REVEAL_WAIT_MS = 2000;

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

/**
 * Scrolling of the grid: when the rows change (new gap, mode or width), keeps
 * the photos at the top of the view in place, before paint so the grid never
 * visibly jumps; and scrolls to the photo `reveal` as soon as the rows show it,
 * highlighting it for {@link HIGHLIGHT_MS}. The rows may still be the old ones
 * when `reveal` is set (a restored photo arrives with the next reload), so it
 * waits a moment before giving up. Returns the highlighted photo and the
 * grid's scroll handler.
 */
export function useGridScroll(
  scrollRef: RefObject<HTMLElement | null>,
  rows: readonly TimelineRow[],
  virtualizer: Virtualizer<HTMLDivElement, Element>,
  reveal: number | null,
  onRevealed: (() => void) | undefined,
) {
  const previousRows = useRef(rows);
  // The scroll position in the old rows. Read from scroll events, because by
  // the time the effect runs, a shorter new canvas may already have made the
  // browser clamp `scrollTop`.
  const lastScrollTop = useRef(0);
  const [highlighted, setHighlighted] = useState<number | null>(null);

  useLayoutEffect(() => {
    const oldRows = previousRows.current;
    previousRows.current = rows;
    virtualizer.measure();
    const element = scrollRef.current;
    if (!element || oldRows === rows) return;
    element.scrollTop = anchoredScrollTop(oldRows, rows, lastScrollTop.current);
    lastScrollTop.current = element.scrollTop;
  }, [rows, virtualizer, scrollRef]);

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (reveal === null || !element) return;
    const target = revealScrollTop(rows, reveal, element.scrollTop, element.clientHeight);
    if (target === null) {
      const timer = setTimeout(() => onRevealed?.(), REVEAL_WAIT_MS);
      return () => clearTimeout(timer);
    }
    element.scrollTop = target;
    lastScrollTop.current = element.scrollTop;
    setHighlighted(reveal);
    onRevealed?.();
  }, [rows, reveal, onRevealed, scrollRef]);

  useEffect(() => {
    if (highlighted === null) return;
    const timer = setTimeout(() => setHighlighted(null), HIGHLIGHT_MS);
    return () => clearTimeout(timer);
  }, [highlighted]);

  const onScroll = (event: UIEvent<HTMLElement>) => {
    lastScrollTop.current = event.currentTarget.scrollTop;
  };
  return { highlighted, onScroll };
}
