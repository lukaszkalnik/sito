# Slice 8 – Reject into "Odrzucone" with undo

**Status:** 🧪 ready for testing (v0.7.2)

## Context

The viewer (slice 7) lets you look at series photo by photo. Now you want to throw away the bad shots. A rejected photo is **moved**, never deleted, into an `Odrzucone` subfolder of its own folder. Scans skip those folders, so the photo disappears from Sito. Every move is logged in SQLite so it can be undone (project rules: never delete, everything undoable). Moving to the system trash becomes an option in slice 11; showing and restoring photos from `Odrzucone` comes in slice 12.

Your decisions:
- **Undo has many steps.** Each "Cofnij" / Ctrl+Z moves back one more rejected photo, newest first, as long as the log has them, also after a restart.
- **Undo is limited to the opened folder.** Cofnij only undoes rejections inside the currently scanned folder and its subfolders. Rejections made in other folders wait until that folder is opened again, so nothing changes out of sight.
- **Rejecting happens in the viewer only**: an "Odrzuć" button and the Delete key. **Undo works in both windows**: a "Cofnij" button and Ctrl+Z.
- **A short toast** in the viewer confirms each rejection, e.g. „IMG_1234.jpg → Odrzucone", with its own "Cofnij" button.

## Design

### Core (`crates/core`)
- **`scan.rs`:** folders named `Odrzucone` (case-insensitive) are skipped like hidden and `.nomedia` folders. Constant `REJECTED_DIR = "Odrzucone"`. Tests cover the skip, including nested ones.
- **`reject.rs`** (new; pure file operations, `thiserror` errors):
  - `reject(path) -> Result<PathBuf>` moves `<dir>/<name>` to `<dir>/Odrzucone/<name>`:
    - it creates the folder if needed
    - it never overwrites: a clash gets `<stem> (2).<ext>`, `(3)`, and so on
    - the move is `fs::rename` (same folder tree, so the same drive, so instant)
  - `restore(from, to) -> Result<()>` moves the file back:
    - it refuses with `Occupied` if `to` exists again, and with `Missing` if the rejected file is gone
    - afterwards it removes `Odrzucone` if the folder is now empty **and was created in this app session** (your request during implementation): an `Odrzucone` folder that existed before stays. It uses `remove_dir`, which never deletes files.
  - tests run in temp folders: the move, a name clash, restore, the occupied and missing cases, and that the empty folder is removed while a non-empty one stays
- **`actions.rs`** (new): `ActionLog` on SQLite (`rusqlite` with the `bundled` feature):
  - table `actions(id INTEGER PRIMARY KEY, at TEXT, kind TEXT, from_path TEXT, to_path TEXT, undone_at TEXT NULL)`
  - `open(path)`, `open_in_memory()` for tests, `record_reject(from, to) -> id`, `last_undoable(under: &Path) -> Option<Action>` (only actions whose `from_path` is inside `under`), `mark_undone(id)`
  - tests: newest first, undone entries are skipped, actions from other folders are ignored (including a sibling whose name only shares a prefix, e.g. `Zdjęcia2` vs `Zdjęcia`), and the log survives reopening the file
- **`timeline.rs`:**
  - `Timeline::remove(path) -> Option<(usize, Photo)>`
  - `Timeline::insert(photo) -> usize` puts a photo back in sort order (the same comparator as `build_timeline`, extracted into `compare_photos`)
  - tests for both

### Tauri (`src-tauri`)
- **Stable photo ids:**
  - `Scan` keeps `ids: Vec<usize>` alongside `timeline.photos`, starting as 0..n
  - removing a photo removes its id; restoring re-inserts the photo with its original id at its sorted position, which keeps ids ascending
  - `AppState::photo(scan_id, id)` looks a photo up by id (binary search), and `get_timeline` sends these ids
  - thumbnail URLs and caches stay valid, so no scan-id bump is needed
  - state tests cover this
- **`commands/reject.rs`:**
  - `reject_photo(id) -> RejectedDto { name }`: core `reject`, then log, then remove from the timeline, then emit `timeline-changed` to all windows
  - `undo_reject() -> Option<UndoneDto { id, name }>`:
    - takes the last undoable action **under the current scan's folder** and calls core `restore`, then marks it undone
    - re-inserts the photo into the timeline, with its old id if known in this scan, otherwise as a fresh id after the largest one
    - it then emits `timeline-changed`
    - with no scan open, there is nothing to undo
  - errors map to `CommandError` codes `occupied` and `missing`; `io` and `internal` already exist
- **The database** lives at `app_data_dir()/sito.db` and is opened in setup. If it cannot be opened, rejecting is refused with an error rather than done without a log.

### UI (`ui/src`)
- **`api/reject.ts`:** `rejectPhoto(id)`, `undoReject()`, `onTimelineChanged(handler)`.
- **`useTimelineEntries`** also reloads entries and series on `timeline-changed`, using a revision counter in its dependencies. The previous data stays on screen meanwhile, as with the gap changes.
- **Viewer:**
  - an "Odrzuć" button in the bar (red outline) and the Delete key reject the shown photo
  - the viewer then shows the next photo in its navigation order, or the previous one at the end, or the message „Brak zdjęć do pokazania." if none is left
  - a **toast** at the bottom reads „{{name}} → Odrzucone" with a "Cofnij" button and disappears after 5 s
  - "Cofnij" (in the bar, in the toast, and Ctrl+Z) undoes the last rejection and **shows the restored photo**
- **Main window:**
  - a "Cofnij" button in the top bar, enabled when there is something to undo (the backend reports `can_undo` in `get_undo_state`, refreshed on `timeline-changed`)
  - Ctrl+Z works there too
  - the grid reloads, and the existing scroll anchoring keeps the view in place
- **Errors:** messages for `occupied` („Nie można cofnąć: w tym miejscu jest już plik o tej nazwie.") and `missing` („Nie można cofnąć: plik nie istnieje już w folderze Odrzucone.") are shown in the toast area.
- **pl.json:** `reject.button` "Odrzuć", `reject.done`, `undo.button` "Cofnij", the error strings, and `viewer.empty`.

### Tests
- **Rust:** listed above, plus serialization of the command DTOs.
- **Vitest:**
  - the viewer rejects on button and Delete, shows the next photo and a toast, and Cofnij calls the API
  - Ctrl+Z works in both windows
  - the hook reloads on `timeline-changed`
- **Playwright:**
  - the mock implements `reject_photo`, `undo_reject` and `get_undo_state` on its generated timeline, with in-memory removal and the `timeline-changed` event
  - a spec rejects in the viewer, sees the next photo and the toast, undoes, and sees the photo back
  - main-window Cofnij is enabled after a rejection and restores the photo in the grid

## Steps (one commit each, pushed)
0. `docs: plan slice 8`: the slice file, ROADMAP 8 → 🚧
1. `feat(core): skip "Odrzucone" folders when scanning`
2. `feat(core): move photos into "Odrzucone" and back`
3. `feat(core): action log in SQLite`
4. `feat(core): remove and re-insert timeline photos`
5. `feat(tauri): reject and undo commands with stable photo ids`
6. `feat(ui): reject in the viewer, undo in both windows`
7. Release v0.7.0, then install it (no installer change), and ROADMAP → 🧪

## Verification
- **Local**, one heavy job at a time: `cargo fmt/clippy/test`, `npm run lint`, `typecheck`, `npm test`, `npm run e2e`.
- **Real app:** a debug build (`tauri build --debug --no-bundle`) on a **copy** of a few photos in the scratchpad (never your gallery). Check:
  - the file system after reject and after undo, with `Odrzucone` created and removed
  - the action log in `sito.db`
  - that a rescan skips `Odrzucone`
- **CI** is green, and the release workflow including the smoke test is green.
- **Manual** (you, v0.7.0), on a test folder:
  - reject a few photos with Delete; the `Odrzucone` subfolders appear
  - press Cofnij several times; the files come back
  - restart Sito and undo again; it still works
  - rescan, and the rejected photos stay hidden

## Risks / notes
- **Folder-scoped undo:** opening a parent folder (e.g. `D:\Desktop`) also lets Cofnij undo rejections made earlier in a subfolder (e.g. `D:\Desktop\Złote gody`), since those photos are part of the open timeline.
- **A rejected file renamed** to `IMG (2).jpg` because of a clash is restored under its original name, since the log stores both paths.
- `rusqlite` with `bundled` compiles SQLite from C. That adds about a minute to clean builds, locally and in CI.

## Follow-up after testing v0.7.0
- Rejecting the second photo of a 2-photo series dissolved the series: the kept photo left "Serie" (it was now a single) and the viewer jumped to the next series. Now series keep their shape: they are found on the scan's photos including those rejected in this session, and a series stays one while any of its photos is left (`series::without_removed`). After "Odrzuć" the viewer stays in the series: next photo of it, else the previous one (`afterReject`).
- A series reduced to one photo now never dissolves (your decision): scans read photos in `Odrzucone` folders into a separate list (`Timeline::rejected`), and series are found including them, so the shape survives rescans and restarts.
- Short series are packed side by side as tinted blocks with compact headers, so a 1-photo series no longer takes a whole row; toasts moved to the top of the window. The Tauri reject and undo logic is tested end to end on copies of the fixture photos.
