---
status: 📝 planned
---

# Slice 28b – Viewer stuck on its first photo

Found by the real-app e2e suite of [slice 28](28-real-app-e2e.md) on CI (2026-11-15). Concept only; the cause is still to be found.

## The bug

Sometimes the viewer window never shows the photo it was opened for: it stays on „Wczytywanie zdjęcia…”. Seen on Linux and Windows CI, a few times in about a dozen runs of `e2e-app.yml`, in the viewer, reject and restart specs, both for the first photo of a new viewer window and for a photo sent to an open one. Never seen on the Mac. Until this is fixed, `e2e-app.yml` fails now and then for this reason, and a failure in those specs on Linux or Windows is most likely this bug.

## What is known

- The failed tests' artifacts (`<n>-viewer.json`, from `e2e-app/support/failure.ts`) show the viewer's backend holding the main window's request (photo id, scan id, view, gap), while `get_viewer_photo` returns `null` for that request.
- The viewer takes `null` as a photo that is not on the timeline yet (`missing` in `useViewerPhoto`) and waits for a change of the timeline that never comes.
- So at that moment one of the conditions in `viewer::page::viewer_photo` fails: the request's scan id is not the backend's current scan, the photo id is not on its timeline, or the photo has no place in the grouping for the request's gap.
- The main window had finished the scan by then (its summary showed all photos and both series). Scan ids are timestamps, so the request's id says when its scan started.

## Next steps

1. Record more on failure: the backend's current scan id and revision and the timeline's ids (a debug-only command, or log lines in `viewer_photo` when it returns `None`).
2. Reproduce: run the viewer spec in a loop on a Linux runner (`workflow_dispatch`) until it fails.
3. Fix the cause with a regression test in `src-tauri` or `core`; whatever the cause, the viewer should not wait forever for a photo of a scan that has been replaced.
