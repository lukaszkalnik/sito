# Slice 13a – Fixes from testing slice 13 (viewer)

**Status:** 🚧 in progress

## Context

Found by the user on 2026-10-30 while testing [slice 13](13-viewer-zoom.md) (together with [slice 34](34-reject-to-trash.md)) in the debug build:

1. **The first photo opened in a fresh viewer window never loads.** The viewer shows the spinner forever; opening the photo again (with the window already open) shows it at once. The backend answers every call quickly, and the photo is an ordinary JPEG (`D:\Pulpit\Złote Gody\Tance\Tance (126).jpg`).

Cause: a freshly created viewer window asks the backend for the photo to show (`take_viewer_request`), and the backend hands it out **once**. The viewer page was reloaded right after loading (its console warns about a callback lost to a reload; in the debug build the Vite dev server reloads pages), so the reloaded page found nothing left and waited for an event that never came.

## Plan

### 1. `fix: keep the viewer's photo request until a newer one replaces it`
- `src-tauri/src/viewer.rs`: `ViewerState` keeps the last request instead of handing it out once; the command `take_viewer_request` becomes `current_viewer_request` (it only reads). A reloaded or late viewer page then still finds its photo. Test: the latest request stays and is replaced by a newer one.
- `ui/src/features/viewer/api.ts`: `takeViewerRequest` → `currentViewerRequest`; `ViewerApp.tsx`, its test and the e2e mock follow.

### 2. `docs: slice 13a ready for testing`
- `docs/domain/` where the viewer hand-over is described, slice and roadmap → 🧪.

Checks: `cargo fmt --check`, `cargo clippy`, `cargo test`, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`. Then, in the debug build (with computer use): open a photo from the timeline into a freshly created viewer window and check it shows.
