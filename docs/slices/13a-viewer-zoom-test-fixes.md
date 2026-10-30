# Slice 13a – Fixes from testing slice 13 (viewer)

**Status:** 🧪 ready for testing (implemented on 2026-10-30)

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

### 3. `fix(ui): holding Delete, Ctrl+Z or Escape acts once, not on every repeat`
- Asked by the user: guard buttons and keys against double actions where they could be destructive. Holding Delete auto-repeats and would reject photo after photo. `shortcuts/keymap.ts` `repeatsWhenHeld`: reject, undo and close ignore `KeyboardEvent.repeat` (the key stays claimed); arrows and zoom keys still repeat.

### 4. `fix(ui): a double-click on Odrzuć, Cofnij, Zamknij or a tile acts once`
- The in-flight guard of „Odrzuć” only covers a reject still running; a move that finishes before the second click of a double-click let that click reject the next photo too (a test reproduced it). New `ui/src/clicks/firstClick.ts` `onFirstClick`: acts only when `MouseEvent.detail <= 1` (keyboard activation is 0). Used on „Odrzuć”, every „Cofnij” (main bar, viewer bar, toast, empty viewer), „Zamknij” and the timeline tiles. „Zamknij” also ignores clicks while the window is closing.
- Not guarded, because repeating them is harmless or wanted: gap −/+, mode toggle, prev/next, strip, „Dopasuj”, the destination picker and its dialog (choosing the trash twice is the same choice), „Wybierz folder” (already disabled while picking or scanning).

### 5. `docs: slice 13a ready for testing`
- `docs/domain/timeline.md` (viewer), slice and roadmap → 🧪.

Checks: `cargo fmt --check`, `cargo clippy`, `cargo test`, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`. Then, in the debug build (with computer use): open a photo from the timeline into a freshly created viewer window and check it shows.

## Verification (2026-10-30)

In the debug build, with computer use: a double-click on „Tance (126)” opened one viewer window with the photo shown at once, and „Zamknij” closed it. All checks green (Rust fmt/clippy/tests, oxlint, Prettier, tsc, Vitest 228, Playwright 47).

**Next:** the user tests the debug build: double-click tiles and „Odrzuć”, hold Delete, together with slices 13 and 34.
