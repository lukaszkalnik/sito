import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type RefObject,
} from 'react';

import { restoredScrollTop, topPosition, type GridPosition } from './gridPosition';
import type { TimelineRow } from './layout';
import { scrollTo } from './scrollTo';

/** How long the grid must stay still before its position is stored. */
export const SAVE_DELAY_MS = 300;

/** Where the grid's position is kept between openings of its folder. */
export interface SavedPosition {
  /** Reads the stored position (`null`: the top); called once when the grid mounts. */
  readonly load: () => GridPosition | null;
  /** Stores the position (`null`: the top). */
  readonly save: (position: GridPosition | null) => void;
}

/**
 * Scrolls once to the stored position when the rows are final (`measured`)
 * and show photos, unless the user scrolled first. Stores the position when
 * scrolling settles, when the grid unmounts and when the page goes away;
 * never before the restore, so the initial top cannot overwrite it. Returns
 * the handler to call on every scroll event.
 */
export function useSavedPosition(
  scrollRef: RefObject<HTMLElement | null>,
  rows: readonly TimelineRow[],
  measured: boolean,
  saved: SavedPosition | undefined,
  lastScrollTop: RefObject<number>,
): () => void {
  const [initial] = useState(() => saved?.load() ?? null);
  const restored = useRef(initial === null);
  const latest = useRef({ rows, saved });
  useLayoutEffect(() => {
    latest.current = { rows, saved };
  });

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (restored.current || !measured || !element || initial === null) return;
    const target = restoredScrollTop(rows, initial);
    if (target === null) return;
    scrollTo(element, target, lastScrollTop);
    restored.current = true;
  }, [rows, measured, initial, scrollRef, lastScrollTop]);

  const flush = useSaveFlush(restored, latest, lastScrollTop);
  return useCallback(() => {
    // A scroll before the restore is the user's own: they stay where they scrolled.
    restored.current = true;
    flush.schedule();
  }, [flush]);
}

/** Stores the current position after {@link SAVE_DELAY_MS}, or right away on unmount or page hide. */
function useSaveFlush(
  restored: RefObject<boolean>,
  latest: RefObject<{ rows: readonly TimelineRow[]; saved: SavedPosition | undefined }>,
  lastScrollTop: RefObject<number>,
) {
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const flush = useMemo(() => {
    const now = () => {
      if (timer.current !== null) clearTimeout(timer.current);
      timer.current = null;
      if (!restored.current) return;
      const { rows, saved } = latest.current;
      saved?.save(topPosition(rows, lastScrollTop.current));
    };
    const schedule = () => {
      if (timer.current !== null) clearTimeout(timer.current);
      timer.current = setTimeout(now, SAVE_DELAY_MS);
    };
    return { now, schedule };
  }, [restored, latest, lastScrollTop]);

  useEffect(() => {
    window.addEventListener('pagehide', flush.now);
    return () => {
      window.removeEventListener('pagehide', flush.now);
      flush.now();
    };
  }, [flush]);
  return flush;
}
