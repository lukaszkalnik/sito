# Slice 36 – Main window follows the viewer

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

From the user on 2026-10-30: while browsing photos in the viewer window, the main window (the grid) should follow, so the photo shown in the viewer is visible in the grid too. Planned to come soon, right after [slice 33](33-split-by-camera.md).

## Concept

- When the viewer shows another photo (stepping with the arrows or the strip, after a reject or an accept), the grid scrolls to that photo and marks it.
- Build on what exists:
  - `revealScrollTop` (`ui/src/features/timeline/layout.ts`) scrolls only when the photo is not fully visible, else it keeps the scroll position.
  - `useGridScroll` highlights a revealed photo, as after undoing a rejection ([Accepting](../domain/accepting.md)).
  - The viewer already tells the backend about its request (`ViewerRequest`) and the main window about the view and gap (`sendViewerContext` / `onViewerContext`); a new event carries the shown photo's id to the main window.

## Open questions

- How the followed photo is marked: a lasting "currently in the viewer" frame while the viewer is open, or the existing short highlight.
- Whether the grid follows always, or only while it is in the same view (mode, gap) as the viewer.
- Fast stepping: throttle the grid's scrolling (e.g. follow the last photo after a short pause) so holding an arrow key stays smooth.
- What happens in the „Serie” view when the viewer shows a single photo the grid hides.

## Related slices

- [Slice 31](31-lazy-grid.md) (lazy grid): revealing a photo must work when its row is not loaded yet.
- [Slice 33](33-split-by-camera.md): the grid and the viewer share the display order, so the followed photo is where the viewer's stepping expects it.

## Verification (outline)

- Stepping through a long series in the viewer keeps the shown photo visible and marked in the grid, without the grid jumping when the photo is already visible.
