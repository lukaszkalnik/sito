# Slice 30 – Load the timeline lazily

**Status:** 📝 concept drafted (plan in plan mode when the slice starts)

## Context

Both windows load the whole timeline at once: `get_timeline` sends every entry (name, folder, relative path, time, time source, format) and `get_series` every series, and the UI works out everything else from that. For the user's Takeout (34,600 photos, measured 2026-10-29 in the debug build, slice 29a item 7) that is 8.5 MB of JSON and about 3.5 s per call, plus a long task of about 180 ms in the webview to parse it.

- **The viewer** loads it on every open, only to show one photo, its series strip and the way to the previous and next photo; until then it shows a spinner (slice 29a; before, a black window). Opening a photo takes about 3–5 s in a large folder.
- **The grid** loads it after every scan, every refresh and every `timeline-changed` (each rejection and undo, and every 30 s during the EXIF pass of slice 29).

## Concept

- **Viewer:** one command returns what the viewer shows for a request (scan, photo id, mode, gap): the photo's entry, its series' entries for the strip, the previous and next photo ids in the current view (also across series in series mode) and the day. Navigation rules (today `ui/src/features/viewer/navigation.ts`, `afterReject`) move to the backend, where the timeline and series already are. Stepping asks again (a few KB per step; previous/next photos are still preloaded).
- **Grid:** the virtualizer needs the row layout (days, series blocks, rows) and only the entries of the rows on screen. The backend could send the layout in a compact form (or the UI a count per day and series) and the entries of a range of rows on request, cached in the UI.
- **Refresh:** `timeline-changed` then only invalidates what is cached; the next visible rows are fetched again.
- Reconsider then whether the viewer window should stay alive (hidden) between opens, which would also save its start-up.

## Open questions

- How the grid's layout is computed without all entries in the UI (in Rust, sent as rows; or counts only).
- What the UI caches and for how long; how stepping through the viewer stays instant.
- What happens to the e2e mocks of `get_timeline` / `get_series`.

## Related slices

- [Slice 29a](29a-fast-scan-test-fixes.md) item 7 (measurements), [slice 14](14-edit-series.md) (editing series changes the layout), [slice 16](16-show-rejected.md) (rejected photos as a view).

## Verification (outline)

- In `F:\Google Photos` (34,600 photos) the viewer opens in well under a second, and stepping stays instant.
- Rejecting, undoing and the EXIF pass's updates no longer move 8.5 MB.
