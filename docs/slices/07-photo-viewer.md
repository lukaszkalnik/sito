# Slice 7 – Photo viewer

**Status:** ✅ done (tested by the user on 2026-10-27 with v0.6.4)

Planned together with [slice 8](08-reject-with-undo.md) (since changed to an `Odrzucone` folder instead of the trash).

## Context

You want Sito's main workflow soon: open a photo full size, see its series, step through photos, and throw away bad shots. This replaces the old roadmap slices 7 ("Compare and decide" with keep/reject decisions held in memory) and 8 ("Quarantine with undo").

The work splits into two releasable slices so you can try the viewer early:
- **Slice 7 – Photo viewer** (your points 1–3) → v0.6.0
- **Slice 8 – Move to trash with undo** (point 4) → v0.7.0

Your decisions:
- **Separate native window**, resizable and reused: clicking another thumbnail shows it in the same window.
- **Previous/next follow the current view.** In "Wszystkie zdjęcia" they walk every photo in timeline order. In "Serie" they walk only series photos, crossing from one series into the next.
- **Trash acts immediately**, with no confirmation. Undo is the safety net: an "Cofnij" button plus Ctrl+Z restores the last trashed photos from the Recycle Bin. Every action is logged locally (project rule).
- **"Rejected" subfolder** instead of the trash comes later, as an option (new slice 11).

**Roadmap after this:**

| # | Slice |
|---|---|
| 7 | Photo viewer |
| 8 | Move to trash with undo |
| 9 | Persistence |
| 10 | Edit series |
| 11 | "Rejected" folder option |
| 12 | Sharpness suggestion |
| 13 | Exposure and weights |
| 14 | Split by content |
| 15 | Series without EXIF date |
| 16 | Faces and eyes |
| 17 | macOS polish |

## Slice 7 – Photo viewer

### Tauri (`src-tauri`)
- **Full-size photo protocol** `photo` (`http://photo.localhost/<id>?scan=<scanId>`):
  - registered like `thumb` in `src-tauri/src/lib.rs` and reuses the URL parsing of `thumbnails::parse_request`
  - reads the original file bytes on a blocking task and serves them as `image/jpeg` with immutable caching
  - WebView2 and WKWebView apply EXIF orientation to `<img>` themselves (`image-orientation: from-image` is the default), so no re-encoding is needed
  - CSP `img-src` adds `photo: http://photo.localhost`
- **Command `open_viewer(id, mode, gap)`:**
  - if no window labelled `viewer` exists, it creates one with `WebviewWindowBuilder` (url `index.html?window=viewer`, title "Sito", 1200×800, resizable)
  - it then emits `viewer-show { id, mode, gap, scanId }` to that window, then `show` + `set_focus`
  - a freshly created window asks for the pending payload once it has loaded (`take_viewer_request`), so the first event cannot be lost
- **Capabilities:** `default.json` covers `["main", "viewer"]`. Events between windows use `core:default`.

### UI (`ui/src`)
- **`main.tsx`** renders `<ViewerApp/>` when `?window=viewer`, otherwise `<App/>`.
- **`features/viewer/navigation.ts`** (pure, unit-tested):
  - `navigationOrder(entries, series, mode)` lists the photo ids in timeline order, all of them in `all` mode and only series members in `series` mode
  - `neighbours(order, id)` returns prev and next
  - `seriesOf(series, index)` returns the series that contains a photo
- **`ViewerApp` / `PhotoViewer`:**
  - loads entries plus `getSeries(gap)` (the existing API) and listens for `viewer-show` and `viewer-context`
  - layout: the photo fills the window (`object-fit: contain`, dark background)
  - top bar: name · time · folder, and the window title shows the file name
  - previous/next buttons sit at the sides; ← → keys also work, and they are disabled at the ends
  - below the photo, a **series strip** shows the thumbnails of the photo's series (reusing `thumbnailUrl` and the tile image logic). The current photo is outlined; clicking a thumbnail shows that photo. There is no strip for single photos.
  - Esc closes the window
- **Main window:**
  - clicking a tile calls `openViewer(entry.id, mode, gap)`
  - Enter on a focused tile does the same; the tile becomes a `<button>` inside its `<li>` for keyboard access
  - when mode or gap change while the viewer is open, `emitTo('viewer', 'viewer-context', { mode, gap })` keeps prev/next in sync
- **Strings in `pl.json`:** `viewer.previous` "Poprzednie zdjęcie", `viewer.next` "Następne zdjęcie", `viewer.close` "Zamknij", `viewer.loadError`.

### Tests
- **Rust:** request parsing for `photo`; content-type and response for a fixture file; `open_viewer` payload serialization.
- **Vitest:**
  - `navigation.ts`, covering both modes, the ends, and a photo that is in no series in `series` mode (it opens anyway, and next goes to the following series)
  - `PhotoViewer`: buttons, keys, strip clicks, and no strip for single photos
  - a tile click calls `openViewer`
- **Playwright:**
  - the mock adds `open_viewer`, events, and a `photo.localhost` route
  - a spec opens the viewer page directly (`/?window=viewer`) with a mocked `take_viewer_request` and checks prev/next across a series boundary in both modes, plus the strip click
  - the main-window spec checks that a click invokes `open_viewer` with the right args

### Steps (one commit each, pushed)
0. `docs: plan slices 7 and 8`: slice files, the roadmap rewrite above, 7 → 🚧.
1. `feat(tauri): serve full-size photos`
2. `feat(tauri): open the viewer window`
3. `feat(ui): photo viewer with series strip and navigation`
4. `feat(ui): open photos from the timeline`
5. `chore: release v0.6.0` (with your OK), then 7 → 🧪.

## Verification
- **Local**, one heavy job at a time: `cargo fmt/clippy/test`, `npm run lint`, `typecheck`, `npm test`, `npm run e2e`. Also run `npm run tauri dev` once to check the second window opens, is reused, and shows rotated photos upright.
- **CI** is green, and the release workflow including the smoke test is green.
- **Manual** (you, v0.6.0):
  - click a photo in "Złote gody"; a resizable window opens
  - the series strip appears, and clicking in it switches the photo
  - ← → cross series boundaries in "Serie" and walk every photo in "Wszystkie zdjęcia"
  - clicking another tile in the main window reuses the open window

## Risks / notes
- **Very large JPEGs** (50 MP+) display at full resolution in the webview. That is fine for viewing, but decoding takes a moment, so the previous photo stays visible until the new one has loaded (no white flash).
- **Recycle Bin restore** depends on the `trash` crate's Windows support (`os_limited`). I'll verify it on CI and locally at the start of slice 8, before building the UI on it.

## Follow-up after testing v0.6.0
- Both windows opened partly below the taskbar and not at the top of the screen. Now they open centred and are shrunk and moved into the monitor's work area (`src-tauri/src/window_fit.rs`).
- Both windows remember their size, position and maximized state across launches (`tauri-plugin-window-state`). A remembered state is fitted to the screen too, e.g. after disconnecting a monitor.
- v0.6.1 still made the viewer jump: the plugin restores a window's state in a task queued on the main thread, after the window is already shown and fitted. Now both windows are created hidden; `window_fit::show_placed` restores the remembered state, fits it to the screen (allowing for the invisible resize borders of Windows frames) and only then shows the window.
- v0.6.2 froze when opening the viewer: `open_viewer` placed the new window from its async command thread while holding the window-state plugin's lock, and the plugin's window registration on the main thread waited for that lock. The placement now runs on the main thread, queued after the registration.
- v0.6.3 did not keep window sizes across restarts: the plugin writes its file only on app exit, which closing the last window did not reach. Sito now writes the state when any window is about to close (`window_fit::save_on_close`). Checked locally with a debug build by closing it like a user (WM_CLOSE) and restarting: a moved and resized window, a maximized one, and one un-maximized again all come back as left.
