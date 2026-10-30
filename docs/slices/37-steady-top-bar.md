# Slice 37 – Steady top bar during background work

**Status:** ✅ done (tested by the user 2026-10-31)

## Context

From the user on 2026-10-30, while testing [slice 33](33-split-by-camera.md): while a scan works in the background, the top bar of the main window grows and shrinks with its status text. The user does not want the bar kept permanently taller just to make room for messages that are only shown for a while.

**Cause:** the bar's status line (`ScanStatus` in `ui/src/features/timeline/TimelineView.tsx`) holds the permanent summary (photos, series, photos without a date, unreadable entries) and the temporary notes. The temporary notes are „Sprawdzanie zmian…” during a background refresh and „Uzupełnianie dat z EXIF: 1234 z 34 600.” during the EXIF pass ([Photos](../domain/photos.md#first-scan-of-a-takeout)). Appending a note makes the line wrap, so the bar grows by a line, then shrinks again when the note goes away. The grid below jumps with it.

## Decisions (with the user, 2026-10-30)

- The temporary notes move out of the bar into a **floating pill at the bottom centre** of the window. The undo toast (`ui/src/features/reject/Toast.tsx`) sits at the top centre just below the bar, so the two never collide and need no stacking.
- The pill must not hide the bottom photo row: the grid always ends with a little empty space, so the last row can be scrolled above the pill.
- The permanent summary stays **on one line** with an ellipsis and the full text as a tooltip (like the folder path above it), so even a narrow window keeps a steady bar.
- Rejected: one line with an ellipsis for everything (cuts off the very progress the note is for); reserving room for a second line all the time (the bar stays large for nothing).

## Plan

### 1. `ScanNotes` pill (new, `ui/src/features/scan/ScanNotes.tsx` + `ScanNotes.css` + `ScanNotes.test.tsx`)

- Props: `refreshing: boolean`, `completing: Completing | null` (type from `scanReducer.ts`).
- Like `Toast`: an always-rendered `div` with `aria-live="polite"` (no `role="status"`, so the bar's summary stays the one status element), containing the pill only when there is a note. Both notes can show together, joined by a space, with the existing strings `scan.refreshing` and `scan.completing` (no new translations).
- CSS: `position: fixed; bottom: 1rem; left: 50%; transform: translateX(-50%)`, `pointer-events: none` (clicks go through to the tiles), smaller and quieter than the toast (semi-transparent dark background, ~0.8rem, rounded pill), `white-space: nowrap`, `font-variant-numeric: tabular-nums` so the counting number does not make the pill wobble.
- Tests (Vitest + Testing Library, by text): nothing shown without notes; the refresh note; the EXIF note with `pl-PL` number formatting; both at once.

### 2. Bar summary on one line (`TimelineView.tsx`, `TimelineView.css`)

- `ScanStatus` drops the two notes. To use the same text as content and tooltip, it builds the summary as one string: `SeriesSummary` becomes a plain helper returning the series text; render `<p role="status" title={text}>{text}</p>`.
- CSS: the status line gets the same `overflow: hidden; white-space: nowrap; text-overflow: ellipsis` as `.timeline-bar-path` (one shared selector, e.g. `.timeline-bar-info p`).
- `TimelineView` renders `<ScanNotes refreshing={summary.refreshing} completing={completing} />` next to `<Toast>`.

### 3. Room under the last row (`TimelineGrid.css`)

- A constant `padding-bottom` on `.timeline` (the pill's height plus its offset, ~3rem). Constant, so nothing moves when the pill appears; only the end of the scroll range gets the space. Check that the virtualizer and the „Cofnij” reveal still behave (the padding is outside `.timeline-canvas`).

### 4. Tests to update

- `ui/src/features/scan/ScanScreen.test.tsx` (refresh test): find „Sprawdzanie zmian…” by text instead of inside the `status` element and assert it disappears; assert the status line itself never contains it.
- `ui/e2e/persistence.spec.ts` and `ui/e2e/scan.spec.ts` (EXIF pass): look for the notes by text (`page.getByText(...)`) instead of in `getByRole('status')`; assert the status line does not contain them.
- `TimelineView.test.tsx`: the status line has its full text as its `title`.

### 5. Docs

- `docs/domain/photos.md` (first scan of a Takeout): the EXIF progress is shown in a floating note at the bottom of the main window, not in the status line; describe „Sprawdzanie zmian…” there too if not yet described.
- `docs/domain/timeline.md`: the main window bar stays one line (summary cut with an ellipsis, full text as a tooltip); background notes float at the bottom centre; the grid ends with space for them.
- This file and the roadmap: 🧪 when implemented.

### Commits (on `main`, pushed)

1. `docs: plan slice 37` (this plan).
2. `feat: background notes float over the grid, bar stays one line` (code, tests, domain docs, slice 🧪).

## Verification

- `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e` (scan and persistence specs).
- Debug build (`npm run tauri dev`), when the user is not scanning: reopen a folder (refresh note) and a Takeout folder (EXIF pass). The bar and grid stay in place while the pill appears, counts up and disappears; the last row can be scrolled above the pill; in a narrow window the summary is cut with an ellipsis and its tooltip shows the full text.

## Implementation (2026-10-31)

- Done as planned: `ScanNotes` pill, one-line summary with tooltip, `padding-bottom: 3rem` on `.timeline`, tests and domain docs ([Timeline](../domain/timeline.md#main-window-bar), [Photos](../domain/photos.md#first-scan-of-a-takeout)). Unit tests and all e2e specs pass.
- The ellipsis rule now covers every line of `.timeline-bar-info`, so the `timeline-bar-path` class was dropped.
- Known limit: „Cofnij” reveals a photo when it lies within the grid's viewport, which includes the pill's area, so a restored photo in the bottom row may sit under the pill while a background note shows.
- Tested by the user in a debug build on 2026-10-31: all OK.
