import type { MouseEvent } from 'react';

/**
 * A click handler that runs `action` only for the first click of a double-
 * or triple-click, so a fast double-click on a button that moves photos (or
 * opens a window) acts once. Activating the button by keyboard (click count
 * 0) always runs it.
 */
export function onFirstClick(action: () => void): (event: MouseEvent) => void {
  return (event) => {
    if (event.detail <= 1) action();
  };
}
