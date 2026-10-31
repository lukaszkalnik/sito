# Slice 40 – Keep the grid position per folder

**Status:** 📝 planned (2026-10-31)

## Context

From the user on 2026-10-31: the grid's scroll position should be kept for each folder between closing and reopening it. Today a folder always opens at the top of the grid, so in a gallery of tens of thousands of photos the user has to find their place again every time.

"Closing" covers both ways a folder goes away:

- choosing another folder („Wybierz inny folder”) and later choosing this one again;
- quitting Sito, which reopens the last folder on start ([Timeline](../domain/timeline.md#main-window-bar)).

## Idea

When a folder opens, the grid starts where the user left it in that folder, instead of at the top.

## What to remember

A pixel `scrollTop` is not enough: between closing and reopening, the rows can differ (another window width, another time gap or mode „Serie”/„Wszystkie”, photos rejected, accepted or changed outside Sito, a background refresh still running). So remember **which photo was at the top of the view**, and how far it was scrolled past, then scroll so that photo is in the same place again:

- **Anchor by file path**, not by photo id: ids belong to one scan and change on the next one.
- **Reuse the anchoring logic** of `anchoredScrollTop` (`ui/src/features/timeline/layout.ts`), which already keeps the top photo in place when the rows change; the new part finds the anchor's tile top in the rows of a newly opened folder.
- **Anchor gone** (rejected or moved since): fall back to the nearest photo that still exists (e.g. the next one in display order, or the closest capture time), else the top.

## Candidate approaches

1. **Frontend, `localStorage`:** a small map folder → `{ path, offset }`, in the data layer next to `lastFolder.ts` and `gapSetting.ts`, saved on scroll (debounced) and when the folder changes or the window closes. Simple; the map needs a size limit (e.g. the 50 folders opened last) so it does not grow forever. The anchor's path must reach the UI: the timeline entries may need to carry it (check what they hold today; keep IPC small).
2. **Backend, SQLite** (next to the photo cache, keyed by folder): survives a cleared webview storage and could later serve slice 15's settings. More code (a command to save, the position returned with the timeline) for data that is only a convenience; a lost position does no harm.

Leaning to 1: the position is a per-machine UI convenience, like the gap setting.

## Open questions

- **When to restore:** the first rows a reopened folder shows come from the cache, before the background refresh ends („Sprawdzanie zmian…”). Restore as soon as the anchor is in the rows, then let `useAnchoredRows` keep it in place while the refresh changes the rows. On a first scan, rows arrive in batches: restore once the anchor appears, but stop waiting (and stay where the user scrolled) as soon as the user scrolls themselves.
- **Interaction with [slice 36](36-grid-follows-viewer.md):** the grid follows the viewer; should the saved position be the one the viewer scrolled the grid to? Probably yes: it is simply the grid's position when the folder closes.
- **Window size:** the grid's width changes how many tiles fit in a row; the anchored photo stays at the top, but the row above it may start with other photos. That is fine.
- **Should the viewer reopen too** (the photo it showed when Sito quit)? Out of scope here; note it as an idea if the user wants it.

## Builds on

- `anchoredScrollTop` and `useAnchoredRows` (`ui/src/features/timeline/layout.ts`, `useGridScroll.ts`): keeping the top photo in place across row changes.
- `revealScrollTop` and the reveal path in `useGridScroll.ts`, which already wait for a photo to appear in the rows.
- Reopening the last folder on start (`ui/src/features/scan/lastFolder.ts`) and [slice 39](39-switch-folder-during-refresh.md) (opening another folder while one is still checked).

## Related slices

- [Slice 31](31-lazy-grid.md) (lazy grid): restoring the position must work when the anchor's row is not loaded yet; the anchor then has to be found by the backend (its index in display order) rather than in the rows the UI holds.
- [Slice 38](38-files-changed-outside.md) (files changed outside Sito): a renamed folder or photo loses its saved position, unless the matching by file identity planned there carries it over.

## Decisions (with the user, 2026-10-31)

- **The view is kept too:** each folder reopens in the view it was left in („Serie” / „Wszystkie”), like its time gap, and at its last position.
- **Storage:** frontend `localStorage` (approach 1), at most the 50 folders used last.

## Plan

### Approach (frontend only, `localStorage`) (frontend only, `localStorage`)

### Data layer: `ui/src/features/timeline/folderView.ts` (new)

- Key `sito.folderView`: `{ [folder]: { mode: TimelineMode, position: GridPosition | null } }`, validated on read (bad entries dropped, like `gapSetting.ts`).
- `loadFolderView(folder): { mode, position }` (defaults: `'series'`, `null`), `saveFolderMode(folder, mode)`, `saveFolderPosition(folder, position)`.
- A save moves the folder to the end of the object and keeps only the **50 folders** used last (folder paths are non-integer keys, so JSON keeps insertion order). Storage failures are ignored with a comment, as in `gapSetting.ts`.

### Domain: `ui/src/features/timeline/gridPosition.ts` (new, pure)

- `type GridPosition = { readonly path: string; readonly takenAt: string; readonly offset: number }`: the photo at the top of the view and how many pixels of its tile were scrolled past.
- `topPosition(rows, scrollTop): GridPosition | null`: the topmost (partly) visible tile, same rule as `anchoredScrollTop`; `null` at the very top (`scrollTop <= 0`) so a folder left at the top reopens at the top.
- `restoredScrollTop(rows, position): number | null`: scroll position putting that photo back at the same offset. Found by `relativePath`; if gone (rejected or moved), the first shown photo not newer than `position.takenAt` (rows are newest first), else the last photo; `null` when the rows show no photo.
- To avoid a third copy of the "walk rows, track tile tops" loop, extract a small exported generator `tilePositions(rows)` (yields `{ entry, top }`) in `layout.ts` and use it in `tileTops`, `anchoredScrollTop`, `revealScrollTop` and the new functions (separate `refactor:` commit first).

### Grid: `ui/src/features/timeline/useSavedPosition.ts` (new hook)

- `useSavedPosition(scrollRef, rows, measured, initial, onSave)`, wired into `useGridScroll` (new target `position: { initial: GridPosition | null; onSave(p: GridPosition | null) }` in `GridScrollTargets`), sharing its `scrollTo` / `lastScrollTop`:
  - **Restore:** a layout effect, once: when `measured` and the rows show photos, scroll to `restoredScrollTop` (before paint, no visible jump). If the user scrolls first, or nothing could be restored, it is done.
  - **Save:** after the scroll settles (~300 ms debounce from the grid's scroll handler, which also sees scrolls caused by following the viewer, revealing and anchoring), and on unmount (flush), save `topPosition(rows, lastScrollTop)`. Nothing is saved before the restore ran, so the initial 0 never overwrites the stored position.
- `TimelineGrid` gets `folder`-bound `initialPosition` / `onPositionChange` props from `TimelineView`; it stays free of storage.

### View: `TimelineView.tsx`

- `useGap(path)` pattern for mode: `useMode(path)` reads `loadFolderView(path).mode` once and saves on change; the initial position is read once per mount (`useState(() => loadFolderView(path).position)`).
- If `TimelineView` nears 300 lines, move `useGap`/`useMode`/`useReveal` into `useFolderView.ts`.

### Docs

- Slice 40 file: decisions + this plan, status 🚧 → 🧪; roadmap in sync.
- `docs/domain/timeline.md`: each folder reopens in its last view and at its last position (by photo, fallback to the nearest by capture time, 50 folders remembered).

## Tests

- Vitest `gridPosition.test.ts`: `topPosition` (top → null, partly scrolled tile, header skipped); `restoredScrollTop` (same rows, another width/mode, photo gone → nearest older, all newer → last, no photos → null); round trip `restoredScrollTop(rows, topPosition(rows, s)) === s`.
- Vitest `folderView.test.ts`: defaults, round trip, invalid data ignored, cap of 50 with most recent kept, storage throwing.
- Vitest `useGridScroll.test.ts`: restores once the width is measured; not after the user scrolled; saves after the debounce (fake timers) and on unmount; no save before restore.
- `TimelineView.test.tsx`: mode stored per folder and restored.
- e2e `persistence.spec.ts`: seed `sito.folderView` with a photo deep in the gallery and „Wszystkie” → the grid opens on that photo in „Wszystkie”; scroll, reload the page (restart) → same top photo.

## Verification

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (unchanged Rust, quick sanity), `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`. Then the user tests a debug build: scroll deep into a large folder, switch to another folder and back, restart Sito; the grid reopens at the same photo and view.

## Commits (each pushed to `main`)

1. `docs: plan slice 40` (plan into the slice file, status 📝 planned).
2. `refactor: share tile positions in the grid layout`.
3. `feat: keep the grid position and view per folder` (+ tests, domain doc, slice 🧪, roadmap).
