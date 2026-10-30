# Slice 13a – Fixes from testing slice 13 (viewer)

**Status:** 🚧 in progress

## Context

Found by the user on 2026-10-30 while testing [slice 13](13-viewer-zoom.md) (together with [slice 34](34-reject-to-trash.md)) in the debug build:

1. **The first photo opened in a fresh viewer window never loads.** The viewer shows the spinner forever; opening the photo again (with the window already open) shows it at once. The backend answers every call quickly, and the photo is an ordinary JPEG (`D:\Pulpit\Złote Gody\Tance\Tance (126).jpg`).

Cause, found with the debug port and computer use: **a double-click on a tile opened two viewer windows.** One click already opens the viewer, so a double-click sends two `open_viewer` calls at once; both found no viewer window and both created one (two webviews labelled `viewer`). The first page took the photo request (handed out only once), the visible one got nothing and showed the spinner; closing it failed („failed to acquire webview reference”).

## Plan

### 1. `fix: keep the viewer's photo request until a newer one replaces it`
- `src-tauri/src/viewer.rs`: `ViewerState` keeps the last request instead of handing it out once; the command `take_viewer_request` becomes `current_viewer_request` (it only reads). A reloaded or second viewer page then still finds its photo (defence in depth; done first, before the real cause was found). Test: the latest request stays and is replaced by a newer one.
- `ui/src/features/viewer/api.ts`: `takeViewerRequest` → `currentViewerRequest`; `ViewerApp.tsx`, its test and the e2e mock follow.

### 2. `fix: open only one viewer window on a double-click`
- `src-tauri/src/viewer.rs`: `show` holds a lock while it looks up or creates the window, so a second call waits and then finds the window the first created. (No unit test: it needs real windows; checked in the debug build.)

### 3. `docs: slice 13a ready for testing`
- `docs/domain/` where the viewer hand-over is described, slice and roadmap → 🧪.

Checks: `cargo fmt --check`, `cargo clippy`, `cargo test`, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`. Then, in the debug build (with computer use): open a photo from the timeline into a freshly created viewer window and check it shows.
