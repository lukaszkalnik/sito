# Slice 8 – Reject into "Odrzucone" with undo

**Status:** 📝 planned (outline; refined when the slice starts)

Context: [slice 7](07-photo-viewer.md#context). Changed on 2026-10-27 at the user's request: rejected photos go into an `Odrzucone` subfolder of their own folder by default instead of the system trash; moving to the trash becomes an option in slice 11.

- **Core:**
  - `reject.rs`: `reject(path)` moves `<dir>/<name>` to `<dir>/Odrzucone/<name>`. It creates the folder, never overwrites (a name clash gets a ` (2)` suffix), and uses a rename, so it is instant on the same drive.
  - `restore(entry)` moves the file back. It refuses if the original path is taken again.
  - An `ActionLog` on SQLite (`rusqlite`, bundled) in the app data folder holds the `actions` table: id, time, kind=`reject`, from, to, size, mtime, undone.
  - The scanner skips folders named `Odrzucone` by default; a scan option lets a later setting include them.
  - Tests use temp folders and an in-memory DB.
- **Tauri:**
  - Photo ids become **stable**: each photo keeps its original index as its id. Removing one does not renumber the rest, so thumbnail URLs and caches stay valid.
  - `reject_photo(id)` and `undo()` update the scan state and emit `timeline-changed` to both windows.
- **UI:**
  - The viewer gets an "Odrzuć" button and the Delete key. After rejecting, it shows the next photo, or the previous one at the end.
  - "Cofnij" (button and Ctrl+Z) works in both windows.
  - The main window reloads, and the existing scroll anchoring keeps the view in place.
- **Never deletes:** files are only moved within their folder, every move is logged, and undo moves them back.
