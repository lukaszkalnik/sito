import { useCallback, useEffect, useRef, useState, type RefObject } from 'react';

import { rankOf, thumbnailQueue, type ScreenRect, type Ticket } from './thumbnailQueue';

/** The part of the screen the timeline shows, or the window outside one. */
function viewOf(element: Element): ScreenRect {
  const timeline = element.closest('.timeline');
  return timeline?.getBoundingClientRect() ?? { top: 0, bottom: window.innerHeight, left: 0 };
}

/**
 * Whether the tile in `ref` may load the thumbnail at `src` now: it waits
 * for its turn in the shared thumbnail queue from when it is rendered, and
 * leaves the queue when it goes away or gets a new `src`. Call `finished`
 * when the thumbnail has loaded or failed, to pass the turn on.
 */
export function useThumbnailTurn(
  ref: RefObject<Element | null>,
  src: string,
): { turn: boolean; finished: () => void } {
  const [turnFor, setTurnFor] = useState<string | null>(null);
  const ticket = useRef<Ticket | null>(null);

  useEffect(() => {
    const element = ref.current;
    if (element === null) return;
    const mine = thumbnailQueue.request(
      () => rankOf(element.getBoundingClientRect(), viewOf(element)),
      () => setTurnFor(src),
    );
    ticket.current = mine;
    return () => mine.release();
  }, [ref, src]);

  const finished = useCallback(() => ticket.current?.release(), []);
  return { turn: turnFor === src, finished };
}
