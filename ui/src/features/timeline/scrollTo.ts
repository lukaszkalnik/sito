import type { RefObject } from 'react';

/** Scrolls `element` to `top` and remembers where it ended up in `lastScrollTop`. */
export function scrollTo(element: HTMLElement, top: number, lastScrollTop: RefObject<number>) {
  element.scrollTop = top;
  lastScrollTop.current = element.scrollTop;
}
