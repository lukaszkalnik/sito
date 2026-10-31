# Slice 40 – Keep the grid position per folder

**Status:** 📝 concept drafted (plan in plan mode when the slice starts)

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
