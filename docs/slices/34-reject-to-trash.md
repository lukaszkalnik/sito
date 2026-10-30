# Slice 34 – Reject into the system trash

**Status:** 📝 planned (plan approved 2026-10-30; implementation next)

## Context

The user wants to move rejected photos to the Recycle Bin ("kosz") now, ahead of the other planned slices. Today a rejected photo always goes into the `Odrzucone` subfolder of its folder ([rejecting.md](../domain/rejecting.md)); the trash was planned as part of the settings screen in slice 15. This slice takes the trash option out of slice 15 and does it on its own, in the main window:

- Next to „Cofnij” in the timeline bar: **„Odrzucone zdjęcia do: [folderu „Odrzucone” | kosza]”**, a `<select>`, with an icon in front of it: a folder icon when the folder is selected, a trash icon when the trash is.
- Choosing „kosza” opens a confirmation dialog first; Cancel keeps the folder.
- While „kosza” is selected the control is prominent: red and bold (and the trash icon red too).
- Default: folder. The choice is remembered across restarts.
- Hard rule still holds: the trash is a move, never a delete, and every trash move is logged and undoable with „Cofnij”.

## Docs and roadmap (first commit, before code)

- This file holds the plan; status 🚧 when implementation starts.
- `docs/ROADMAP.md`: row 34 right after 13 (so it is next).
- `docs/slices/15-settings.md`: drop the trash option from its idea, link to slice 34.
- At the end: `docs/domain/rejecting.md` describes the trash destination (domain = how it behaves today).

## Backend (core)

Dependency: **`trash` 5.2.9** (MIT, Arturs Kovacs; uses `windows` 0.62, already in our tree via Tauri). On Windows it moves via `IFileOperation` with `FOF_ALLOWUNDO`; `os_limited::list()` and `restore_all()` (Windows/Linux only) give undo. macOS has no restore API in the crate → undo there returns an error code for now (macOS restore belongs to slice 23).

1. **`crates/core/src/reject.rs`**: extract `photo_files(path) -> Vec<PathBuf>` (photo, RAW companion, their own Takeout date files) from the logic in `prepare` (`find_companion`, `find_own_sidecar`), so folder and trash agree on what belongs to a photo. Tests for it.
2. **New `crates/core/src/trash.rs`** (`TrashError` via thiserror):
   - `trash_photo(path) -> Result<Vec<PathBuf>, TrashError>`: trashes all `photo_files` in one `trash::delete_all` call (date files first is not needed: it is one shell operation).
   - `restore_from_trash(files: &[PathBuf], trashed_after: i64)` (Windows/Linux): `os_limited::list()`, pick for each logged path the newest bin item with that original path deleted at/after the action time (pure function `pick_items(items, files, after)` — unit-tested with fake items), refuse with `Occupied` if a target exists, `Missing` if the photo is not in the bin, then `restore_all`. Other files missing from the bin → reported, like `Restored::errors` today.
   - macOS: `restore_from_trash` returns `TrashError::RestoreUnsupported`.
   - Real trash calls are not run in unit tests (no touching the real Recycle Bin); one `#[ignore]` test on a temp file for manual runs.
3. **`crates/core/src/actions.rs`**: new kind `"trash"`. `record_trash(from, files)` writes the action (`from_path` = `to_path` = photo path, since the bin has no path we choose) plus `action_moves` rows for the other files. `last_undoable` returns both kinds; `RejectAction` gets `place: Placement` with `enum Placement { Folder { to, extra: Vec<FileMove> }, Trash { files: Vec<PathBuf>, at: i64 } }` (illegal combinations unrepresentable). No schema migration needed (kind is already a column); tests for both kinds, and old folder entries still read.

## Backend (src-tauri)

4. **`src-tauri/src/reject.rs`**: `reject_photo(id, destination)` with `#[serde(rename_all = "camelCase")] enum Destination { Folder, Trash }`. Trash path: log first (`record_trash`), then `trash_photo`, `forget` on failure — same order as the folder path. Undo dispatches on `Placement`. Trash and undo-from-trash run via `off_main_thread` (COM work and listing the bin can take a moment). The file is ~460 lines with tests; split into `reject/folder.rs` and `reject/trash.rs` submodules as a separate `refactor:` commit first.
5. **Safety: drives without a Recycle Bin.** On removable (USB stick, SD card) and network drives Windows has no bin and would delete permanently. Before trashing, check the drive type of the photo's folder (`GetDriveTypeW`; only `DRIVE_FIXED` is allowed) and otherwise refuse with the new error code `noTrash` ("Ten dysk nie ma kosza – wybierz folder „Odrzucone”."). This needs one `unsafe` FFI call: workspace lint `unsafe_code` goes from `forbid` to `deny`, with a single `#[expect(unsafe_code, reason = "...")]` in a tiny `src-tauri/src/reject/drive.rs`. (Windows still shows its own warning if a file is too large for the bin, thanks to `FOF_WANTNUKEWARNING` in the crate.)
6. **`src-tauri/src/error.rs`**: new codes `noTrash`, `trashUndoUnsupported`.
7. Licenses: add `trash` (MIT, already allowed) → `npm run notices:rust`, commit `ui/notices/rust-crates.txt`.

## Frontend

8. **`ui/src/features/reject/destination.ts`** (data): `type RejectDestination = 'folder' | 'trash'`, `loadDestination()` / `saveDestination()` in `localStorage` (validated, default `'folder'`), like `timeline/gapSetting.ts`. Tests.
9. **`ui/src/features/reject/useRejectDestination.ts`**: current value, kept in sync across the main window and the viewer via the `storage` event.
10. **`ui/src/features/reject/DestinationPicker.tsx`** (+ `.css`): label „Odrzucone zdjęcia do:”, `<select>` with „folderu „Odrzucone”” / „kosza”, inline SVG folder/trash icon (`aria-hidden`, the select carries the label). Trash selected → class making select + icon red and bold (colour token, readable in dark mode).
11. **`ui/src/features/reject/TrashConfirmDialog.tsx`**: native modal `<dialog>` like `about/LicensesDialog.tsx`: „Przenosić odrzucone zdjęcia do kosza?” / „Zdjęcia trafią do Kosza systemu Windows. Można je przywrócić przyciskiem „Cofnij” albo z Kosza, dopóki nie zostanie opróżniony.” Buttons „Anuluj” (default focus, Escape) and „Tak, do kosza”. Only switching *to* the trash asks; switching back to the folder does not.
12. **`TimelineView.tsx`**: picker placed before „Cofnij” in the bar.
13. **Viewer**: `ViewerApp.tsx` passes the destination to `rejectPhoto(id, destination)`; toast „{{name}} → Kosz” vs. „→ Odrzucone”; the „Odrzuć” button hint follows the destination („Przenieś zdjęcie do kosza (Delete)”). Undo messages: `missing` for trash → „plik nie jest już w Koszu.”, plus texts for `noTrash`, `trashUndoUnsupported`.
14. All strings in `ui/src/i18n/pl.json`.

## Tests

- Rust: `photo_files`, `pick_items` (newest match, ignores older bin items with the same path, missing photo), action log round-trip of both kinds, command-level tests for the folder path unchanged; trash command tests with the bin calls behind a small trait/closure so they do not touch the real bin.
- Vitest: `destination.ts`, `DestinationPicker` (choosing „kosza” opens the dialog; cancel keeps folder; confirm switches and shows the red/bold state; going back to folder needs no confirmation), viewer passes the destination and shows the right toast.
- Playwright: main window flow (pick trash → confirm → control is emphasised; reload keeps it), with mocked commands.

## Verification

1. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`.
2. Debug build (`npm run tauri dev`), on a copy folder on the internal disk: choose „kosza”, confirm, reject 2–3 photos (incl. a RAW+JPEG pair) → they are in the Recycle Bin with their date files; „Cofnij” brings them back in order; restart the app and undo still works.
3. Folder on a USB stick: rejecting with „kosza” is refused with the `noTrash` message, nothing is deleted.
4. Switch back to the folder → rejects go to `Odrzucone` again.

## Commits (each pushed to `main`)

`docs:` slice 34 + roadmap → `refactor:` split src-tauri reject module → `feat(core):` photo_files + trash module → `feat(core):` trash actions in the log → `feat:` reject to trash command + drive check → `feat(ui):` destination setting, picker and confirm dialog → `feat(ui):` viewer uses the destination → `test(e2e):` → `docs:` domain + slice 34 ready for testing (🧪). Then pause for testing.
