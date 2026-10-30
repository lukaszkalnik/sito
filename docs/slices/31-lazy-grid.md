# Slice 31 – Load the grid lazily

**Status:** 📝 concept drafted (plan in plan mode when the slice starts)

## Context

Split out of [slice 30](30-lazy-timeline.md) on 2026-10-29; slice 30 makes only the viewer lazy. The grid still loads the whole timeline (`get_timeline`, 8.5 MB of JSON and about 3.5 s for the user's 34,600-photo Takeout) after every scan, every refresh and every `timeline-changed`: each rejection and undo, and every 30 s during the EXIF pass of slice 29.

## Concept

- The virtualizer needs the row layout (days, series blocks, rows), but only the entries of the rows on screen. The layout depends on the width (blocks flow into rows), so the backend could send a compact form instead: the days as position ranges (`date`, `start`, `len`), the series, and whether the scan spans several folders. The UI's `buildRows` then builds rows over position ranges rather than entries.
- The entries of the visible rows are fetched by position range (for example in chunks of a few hundred) and cached in the UI.
- **Refresh:** `timeline-changed` only invalidates the cache, and the visible rows are fetched again.
- Anchoring and revealing (`anchoredScrollTop`, `revealScrollTop`) work on positions: these stay the same across gap and mode changes. A restored photo's position comes from the backend.
- Following the viewer ([slice 36](36-grid-follows-viewer.md)) also reveals a photo by id, often one whose row is not loaded yet: `viewer-shown` could carry the photo's position, or the grid asks the backend for it, as for a restored photo.

## Builds on

- Slice 30's backend series cache (`cached_series` in `src-tauri/src/series.rs`, keyed by gap and scan revision) and the `Scan` accessors `position_of` / `entry_at`.

## Open questions

- Chunk size, and how long the UI keeps chunks.
- How anchoring behaves across a reject (positions shift by one).
- What happens to the e2e mocks of `get_timeline` / `get_series`.

## Related slices

- [Slice 30](30-lazy-timeline.md), [slice 14](14-edit-series.md) (editing series changes the layout), [slice 16](16-show-rejected.md) (rejected photos as a view).

## Verification (outline)

- In `F:\Google Photos`, rejecting, undoing and the EXIF pass's updates no longer move 8.5 MB, and the grid stays smooth while scrolling fast.

## Builds on

- [Slice 33](33-split-by-camera.md): `get_series` returns the display order with the series; a lazy grid must take the order into account when loading entries by range.
