# Slice 8 – Move to trash with undo

**Status:** 📝 planned (outline; refined when the slice starts)

Context and decisions: [slice 7](07-photo-viewer.md#context).


- **Core:**
  - `actions.rs` holds an `ActionLog` on SQLite (`rusqlite`, bundled) in the app data folder. The `actions` table has id, time, kind=`trash`, path, size, mtime, undone.
  - a `Trash` trait wraps the `trash` crate (`delete`; `os_limited::restore_all` on Windows/Linux), with a fake for tests
  - `trash_photo(log, trash, path)` and `undo_last(log, trash)` are tested with the fake and an in-memory DB
  - restoring on macOS returns `Unsupported` for now (the slice 17 polish handles it)
- **Tauri:**
  - photo ids become **stable**: each photo keeps its original index as its id. Removing one does not renumber the rest, so thumbnail URLs and caches stay valid.
  - `trash_photo(id)` and `undo()` update the scan state and emit `timeline-changed` to both windows
- **UI:**
  - the viewer gets a "Przenieś do kosza" button and the Delete key
  - after trashing, it shows the next photo (or the previous one at the end)
  - an "Cofnij" button and Ctrl+Z work in both windows
  - the main window reloads, and the existing scroll anchoring keeps the view in place
- **Never deletes:** files only go to the system trash, every move is logged, and undo restores to the original path.

