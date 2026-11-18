import type { MouseEvent } from 'react';

/** What a context menu of the grid is for: a photo, or a series by its first photo. */
export type MenuTarget =
  | { readonly kind: 'photo'; readonly id: number }
  | { readonly kind: 'series'; readonly firstId: number };

/** A context menu asked for in the grid, with where to show it (window pixels). */
export interface MenuRequest {
  readonly target: MenuTarget;
  readonly x: number;
  readonly y: number;
}

/**
 * The menu request for `target` from a `contextmenu` event. A menu opened
 * from the keyboard (Menu key, Shift+F10) has no pointer position, so it
 * opens below the element instead.
 */
export function menuRequest(target: MenuTarget, event: MouseEvent<HTMLElement>): MenuRequest {
  if (event.clientX !== 0 || event.clientY !== 0) {
    return { target, x: event.clientX, y: event.clientY };
  }
  const rect = event.currentTarget.getBoundingClientRect();
  return { target, x: rect.left, y: rect.bottom };
}
