# Slice 36 – Main window follows the viewer

**Status:** 🧪 implemented, ready for testing (2026-10-31)

## Context

From the user on 2026-10-30: while browsing photos in the viewer window, the main window (the grid) should follow, so the photo shown in the viewer is visible in the grid too. The viewer often covers the grid, so after the viewer closes, the last photo shown should pulse for a while so the user finds their place.

## Concept

- When the viewer shows another photo (stepping with the arrows or the strip, after a reject or an accept), the grid scrolls to that photo and marks it.
- Build on what exists:
  - `revealScrollTop` (`ui/src/features/timeline/layout.ts`) scrolls only when the photo is not fully visible, else it keeps the scroll position.
  - `useGridScroll` highlights a revealed photo, as after undoing a rejection ([Accepting](../domain/accepting.md)).
  - The viewer already tells the backend about its request (`ViewerRequest`) and the main window about the view and gap (`sendViewerContext` / `onViewerContext`); a new event carries the shown photo's id to the main window.

## Decisions (with the user, 2026-10-31)

- **Marking:** a lasting accent frame on the tile while the viewer is open, moving with each photo; when the viewer closes, that tile pulses for some time (about 5 s).
- **Scrolling:** only when the photo is not fully visible (`revealScrollTop` centres it then). The frame moves immediately; the scroll waits for a short pause (~150 ms) so holding an arrow key stays smooth.
- **Same view:** no setting needed: the main window already pushes its mode and gap to the viewer (`sendViewerContext`), so both always show the same view.
- **Photo hidden in the grid** (e.g. a single photo while „Serie” is shown after its series dissolved): no frame, no scroll; nothing else happens.

## Related slices

- [Slice 31](31-lazy-grid.md) (lazy grid): revealing and following a photo must work when its row is not loaded yet.
- [Slice 33](33-split-by-camera.md): the grid and the viewer share the display order, so the followed photo is where the viewer's stepping expects it.

## Plan

### Events (data layer)

- `ui/src/features/viewer/api.ts`: new event `viewer-shown` with `{ scanId, id }`, sent by the viewer to the main window (`emitTo('main', …)`): `sendViewerShown(shown)` / `onViewerShown(handler)`.
- Viewer closing: the backend emits `viewer-closed` to `main` on the viewer window's `WindowEvent::Destroyed` (also covers the OS close button and Alt+F4). New small handler in `src-tauri/src/viewer.rs` (`viewer::on_window_event`), registered in `lib.rs` together with `window_fit::save_on_close` (one `on_window_event` closure calling both). TS wrapper `onViewerClosed(handler)` in `viewer/api.ts`. No new capability needed (`core:default` covers emitting and listening).

### Viewer side

- `ui/src/features/viewer/ViewerApp.tsx` (`ReadyViewer`): an effect sends `viewer-shown` whenever the ready photo's `entry.id` (or scan) changes. Errors are logged, not swallowed.

### Main window side

- New hook `ui/src/features/timeline/useFollowViewer.ts`: listens to `viewer-shown` (ignores another `scanId`) and `viewer-closed`; state as a discriminated union `{ kind: 'none' } | { kind: 'open'; id }`, with a pure transition function `followReducer` next to it (unit-tested). Returns `followed: number | null` (the id while the viewer is open); on close it hands the last id to the reveal, so the closing pulse reuses the reveal path (scroll if hidden + highlight).
- `TimelineView.tsx`: wires the hook and passes `followed` through `TimelineBody` to `TimelineGrid`. `useRevealRestored` becomes a general `useReveal` (a restored photo or the one last shown in the viewer).
- Longer pulse after close: the reveal carries a reason (`'restored' | 'viewerClosed'`); a restored photo pulses twice as today, the photo last shown in the viewer pulses for about 5 s (modifier class `timeline-tile--highlighted-long`, matching duration in `useGridScroll`).
- `useGridScroll.ts`: new `follow` input. A layout effect keyed on `follow` and `rows` scrolls via `revealScrollTop` after a 150 ms debounce (cleared on the next step), without highlighting. A shared helper sets the scroll target and remembers `lastScrollTop` for both reveal and follow.
- `TimelineRowView.tsx` / `TimelineTile.tsx`: new `followed` prop → class `timeline-tile--followed` and `aria-current="true"`; the row context gains `followed`.
- `TimelineGrid.css`: `.timeline-tile--followed` draws a steady inset accent frame (like the highlight, via `::after`), keeping the accepted check visible.

### Docs

- `docs/domain/timeline.md`: the grid following the viewer (frame, scroll only when hidden, debounce, pulse after close).
- [Slice 31](31-lazy-grid.md): note that following the viewer, like revealing, must work when the row is not loaded yet.

### Tests

- Vitest: `followReducer` transitions (shown, another scan ignored, closed → reveal id); `useGridScroll` following: no scroll when visible, scroll after the debounce with fake timers, only the last of fast steps scrolls; the followed tile is marked (`aria-current`).
- `ViewerApp.test.tsx`: sends `viewer-shown` for the shown photo and after stepping.
- e2e: `emitTauriEvent` takes a payload; emitting `viewer-shown` for a photo far down scrolls the grid to it and marks it; `viewer-closed` removes the frame and pulses the tile.

## Implementation notes

- `followReducer` (`ui/src/features/timeline/followViewer.ts`) keeps the photo to reveal when a close is reported twice (the e2e mock delivers events to both listeners React's StrictMode registers).
- The followed frame is drawn by `::before`, so it never cancels the highlight pulse on `::after`.
- The pulse lengths per reason live in `HIGHLIGHT_MS` (`ui/src/features/timeline/reveal.ts`).

## Verification

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`.
- Debug build, tested by the user: stepping through a long series with the arrows keeps the shown photo framed and visible in the grid, without jumps when it is already visible; holding an arrow stays smooth; closing the viewer pulses the last photo.
