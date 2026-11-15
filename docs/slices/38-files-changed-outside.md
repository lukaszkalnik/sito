---
status: 🧪 ready for testing
---

# Slice 38 – Files changed outside Sito

Plan approved by the user 2026-11-15; built 2026-11-15, waiting for the user's test. What it established is in [Timeline](../domain/timeline.md#files-changed-outside-sito), [Photos](../domain/photos.md#moved-and-renamed-photos), [Accepting](../domain/accepting.md) and [Rejecting](../domain/rejecting.md).

## Context

From the user on 2026-10-31, while testing [slice 36](36-grid-follows-viewer.md): they renamed the open folders from „Zdjęcia z roku …” to just the year. Stepping to the next photo in the viewer then showed „Nie udało się wczytać zdjęcia.”

**Cause:** a scan keeps every photo's path in memory, and nothing watches the disk. After a rename, a move or a delete outside Sito:

- the viewer and the not-yet-loaded thumbnails ask for files at their old paths, which fail with the generic load error ([Timeline](../domain/timeline.md#viewer));
- rejecting such a photo fails as well;
- data kept by path no longer matches: accepted marks ([Accepting](../domain/accepting.md)), the photo cache (the renamed photos are read again in full on the next scan, like new ones) and the undo log of earlier rejections ([Rejecting](../domain/rejecting.md)).

Choosing the folder again („Wybierz inny folder”) fixes the timeline today, but nothing tells the user to do that.

## Idea

Sito notices that photos changed on disk and brings the timeline up to date, or at least says what happened and offers a rescan, instead of showing a load error.

**Rescan button (asked by the user, 2026-10-31):** whatever else the slice does, the main window's bar gets a button „Odśwież” to rescan the open folder on demand, e.g. right after the user changed files in Explorer. It runs the same background refresh as a reopened folder: the grid stays usable, „Sprawdzanie zmian…” shows meanwhile, and only new or changed files are read. A shortcut for it goes into `ui/src/shortcuts/keymap.ts` (F5 is the usual key for refreshing). It is disabled while a scan or refresh is running.

## Candidate approaches

1. **Detect on failure.** When the photo protocol or a reject cannot find a file, it reports a distinct error code (`missing`). The viewer and the grid then show a message that the file is no longer in the opened folder with a button „Odśwież folder”, which runs the background refresh that already runs when the last folder is reopened on start („Sprawdzanie zmian…”, [Timeline](../domain/timeline.md#main-window-bar)). Small, and no background work while nothing changes.
2. **Refresh on focus.** When the main window or the viewer gets focus again (the user was in Explorer), run the background refresh. It catches changes before the user hits them, but costs a walk of the folder (about how long for 34,600 photos?) on every return to the app.
3. **Watch the folder** (`notify` crate, ReadDirectoryChangesW on Windows, FSEvents on macOS) and refresh after changes settle. The most immediate, but a new dependency, many events for a large rename, and watcher limits on network drives.

Approaches can combine: the rescan button always, 1 as the safety net (its „Odśwież folder” does what the button does), plus 2 or 3.

## Open questions

- **Keeping data across a rename:** should accepted marks and the photo cache follow a renamed folder? A refresh could match a vanished file to a new one by size, mtime and name (the cache's file identity without the path) and carry its mark and cached metadata over, so a renamed folder of 34,600 photos is not read again.
- **The undo log:** an earlier rejection's original path may no longer exist (its folder was renamed). Restore it to the old path (recreating the folder), or to the folder's new name when it can be matched?
- **Undo blocked by a taken name** (from the user, 2026-11-08): when another file now has a rejected photo's original name, undo fails with „Nie można cofnąć: w tym miejscu jest już plik o tej nazwie.” and the entry stays on top of the stack, so „Cofnij” keeps failing and every older rejection behind it is out of reach until the user frees that name by hand. Unlike a missing file (`Missing`, `MissingFromTrash`), `Occupied` is not skipped (`undo_reject` in `src-tauri/src/reject.rs`). Options: restore under a free name (`<name> (2)`, as rejecting already does in `Odrzucone`; for a RAW+JPEG pair all files keep one number) and say so in the toast, or skip the entry after telling the user. The first keeps every rejection undoable and needs no new decision from the user.
- **The viewer during a refresh:** the timeline ids change after a refresh, so the viewer needs to find its photo again (by path or the matched identity), like it does after a rejection.
- **The followed photo** ([slice 36](36-grid-follows-viewer.md)) goes away with the old ids; the grid should follow the viewer's photo again once it is found.

## Builds on

- The background refresh of a reopened folder and its note „Sprawdzanie zmian…” ([Timeline](../domain/timeline.md#main-window-bar)).
- [Slice 39](39-switch-folder-during-refresh.md): the refresh stops when another folder is opened and stores what it read every 200 photos, so „Odśwież” can start one at any time and a refresh of a renamed folder can be interrupted without losing its work.
- Error codes of commands mapped to messages in `pl.json` (CLAUDE.md, Errors).

## Plan

**Decisions:** the rescan button always; detection on failure as the safety net (candidate 1), user-driven: Sito says what happened and offers „Odśwież”, it never rescans on its own (no refresh loops when a USB drive is unplugged, no walk on every focus). No folder watcher (candidate 3) and no refresh on focus (candidate 2) for now. Every scan recognises moved and renamed photos, also when the scanned folder itself was renamed or moved and is opened under its new path, so their cache entry and accepted mark carry over (and, in a refresh, their timeline id). Undo blocked by a taken name restores under a free name.

**Moved photo:** a photo that is gone from its old path while a file with the same **file name, size and modification time** shows up at a new path, in the opened folder or anywhere else the photo cache knows (so a renamed scanned folder is covered too). Only unique matches count: if two vanished photos or two new files share that identity, none of them is matched (they are read again as new, as today). A cached photo whose old path **still exists** is a copy, not a move: its cached metadata is reused (it is the same file), but its mark stays where it is. A photo without a modification time is never matched. Its Takeout date file and RAW companion must match by size and modification time too (their paths change with the photo), or the photo is read again.

### Core (`crates/core`)

1. **`moved.rs`** (new, pure): `FileIdentity { name: OsString, size: u64, modified: SystemTime }` (`TryFrom<&Photo>` / from a found file; `None` without mtime) and `match_moved(gone, appeared) -> Vec<(usize, usize)>` returning index pairs of unique identity matches. Tests: rename, move to another subfolder, ambiguous duplicates, no mtime, changed size.
2. **Photo cache reuses moved photos.** `split_cached` (`timeline.rs`): files not found by path are matched in two steps: first against the cached photos under the root that the walk did not find (`match_moved`, no file system access); the rest are looked up in the whole cache by identity (`PhotoCache::by_identity`, an index on size and modification time, added with `CREATE INDEX IF NOT EXISTS`, so the cache is not emptied), keeping candidates of the same file name; a candidate whose path no longer exists is a move, one whose path still exists a copy (checked with one `exists()` per candidate, never for the whole cache). Either way the match is reused as a cached photo with the new paths (photo, date file, companion; `is_unchanged` ignoring paths), so a renamed folder of 34,600 photos is not read again. The moved photos are stored under their new paths (`store`); `forget_missing` drops the old ones as today. `build_timeline_cached` returns the moves (old path → new path; copies are not moves) alongside the timeline. Tests (`tests/photo_cache.rs`): rename a subfolder of a cached gallery, rescan: nothing is read (the `Reading` total is 0), the timeline holds the photos at their new paths and the moves are returned; rename the gallery root and scan the new path: the same; copy the gallery and scan the copy: nothing is read, no moves.
3. **Marks follow.** `ActionLog::move_accepted(&[(PathBuf, PathBuf)])` (`accept.rs`) moves accepted marks from old to new paths in one transaction. Test.
4. **Undo restores under a free name.** `restore` (`reject.rs`): when the original name is taken, the photo (and its companion, with the same number) goes back as `<name> (2)` etc. via `free_names`; its date files follow the new names (as rejecting already names them). `Restored` reports the path it went back to. Tests: taken name → `(2)`; RAW+JPEG pair with one taken → both `(2)`; the file that took the name is untouched. `RejectError::Occupied` then only remains for the exclusive-move race.
5. **Rejecting a vanished photo** reports a distinct `RejectError::PhotoGone(path)` before anything is logged or moved (today it fails as a generic I/O error after logging). Test.

### Tauri (`src-tauri`)

6. **Ids and marks follow the moves.** Every scan (first scan, reopened folder, refresh) passes the moves from step 2 to `move_accepted`. `Scan::refresh` (`scan/state.rs`) takes them too: a moved photo keeps its id, so the viewer, the followed tile ([slice 36](36-grid-follows-viewer.md)) and the strip find it again. Tests in `state.rs`.
7. **`refresh_folder` command:** rescans the current folder in the background like a reopened one: `begin_scan` (stops a running EXIF pass, which the refresh restarts at its end), emits `scan-refreshing` with the scan id, then `refresh_in_background`. No-op without a scan.
8. **Vanished files are reported.** The `thumb:` and `photo:` protocols answer `410 Gone` when the photo's file no longer exists (instead of 404 for every I/O error), and emit `photos-missing` (scan id) at most once per timeline revision. When the scanned folder itself is gone (renamed, moved or its drive unplugged), they emit `folder-missing` instead, and so does a refresh that fails with `ScanError::NotFound` (today it keeps the cached view silently). New command `photo_missing(id) -> bool` for the viewer (an `<img>` error does not tell the status). `CommandError::PhotoMissing` (`photoMissing`) for `reject_photo` (step 5). `UndoneDto` gets the restored name when it differs from the original (step 4).

### UI (`ui`)

9. **„Odśwież” in the main window bar:** an icon button (the usual refresh icon: one circular arrow with an arrowhead, ↻, accent colour) right of the folder button, name „Odśwież”, tooltip „Sprawdź zmiany w folderze (F5)”; shortcut `refresh` = F5 in `keymap.ts`. Disabled while picking, scanning or refreshing. Calls `refreshFolder`; the reducer gets `refreshStarted` (from `scan-refreshing`, so a refresh started in the viewer shows too), which sets `summary.refreshing`, and the pill shows „Sprawdzanie zmian…” until `scan-refreshed`.
10. **Missing files in the main window:** on `photos-missing` the pill shows „Niektórych zdjęć nie ma już w otwartym folderze – zostały przeniesione, zmieniły nazwę lub usunięto je.” with a button „Odśwież”, until a refresh starts. **Tiles that failed** load again after the timeline changes (the error state is reset by the timeline revision; error responses are not cached), so a moved photo's tile shows its thumbnail after the refresh.
10a. **The scanned folder is gone** (`folder-missing`): instead of the missing-photos note, the main window shows a notice under the bar: „Otwartego folderu nie ma już na dysku – zmieniono jego nazwę, przeniesiono go lub odłączono dysk.” with the button „Wybierz inny folder” (the folder picker, as the folder button). Choosing the folder under its new name finds its photos in the cache (step 2): nothing is read again and the accepted marks carry over. „Odśwież” stays enabled, so a reconnected drive can be checked again; the grid keeps showing the cached photos meanwhile. A refresh that finds the folder again clears the notice. In the viewer, a photo that fails to load because the folder is gone shows „Otwartego folderu nie ma już na dysku. Wybierz go ponownie w oknie głównym.” (no button: picking a folder belongs to the main window). Opening the last folder on start when it is gone already ends in „Nie znaleziono wybranego folderu.”; that stays.
11. **Viewer:** when the photo fails to load and `photoMissing` says it is gone, it shows „Tego zdjęcia nie ma już w otwartym folderze – zostało przeniesione, zmieniło nazwę lub usunięto je.” with „Odśwież folder” (runs `refresh_folder`) instead of the generic error. A failed photo loads again when the timeline changes. Rejecting a vanished photo shows the same message in the toast. After the refresh, a moved photo shows again (same id); a deleted one goes the way a rejected photo does today (the viewer moves on to the photo that took its place).
12. **Undo toast** for a restore under a new name: „Przywrócono <name> jako <new name> – nazwa była zajęta.”
13. Strings in `pl.json`; Polish plurals not needed here.

### Docs

14. `docs/domain/timeline.md` (bar, pill, viewer), `photos.md` (moved photos keep their cache entry), `accepting.md` (marks follow a moved photo, also into a renamed scanned folder; unique matches only; copies keep no mark), `rejecting.md` (undo under a free name; rejecting a vanished photo). This file and the roadmap: 🧪 once built.

### Not in this slice

- Refreshing on focus or watching the folder (candidates 2, 3): a later slice if the button and the safety net are not enough.
- A rejection whose `Odrzucone` folder was renamed along with its parent: undo skips it as „missing” (today's behaviour); matching undo log entries to moved folders is left out.
- Opening a renamed scanned folder automatically: Sito cannot know the new name, so the user chooses it (step 10a).

## Implementation notes

Built as planned, with these differences:

- `check_photo_file(id)` (instead of `photo_missing(id) -> bool`) answers `null`, `"photo"` or `"folder"`, since the viewer shows a message of its own for a vanished folder.
- Moves into or out of an `Odrzucone` folder are not reported as moves (a rescan after Sito's own rejection would otherwise move the accepted mark into `Odrzucone`, and undo would bring the photo back without it); their cached metadata is still reused.
- The photo cache stores relocated photos and forgets the old paths of moves only once a scan is complete, so a stopped scan still reports the moves (and moves the marks) next time.
- F5 also refreshes from the viewer window (so it never reloads either window), and a photo restored under a new name by undo takes its accepted mark along.
- A photo that left the timeline in a refresh (deleted outside Sito) makes the viewer move on to the photo the backend picked for after rejecting it.
- The background refresh moved to `src-tauri/src/scan/refresh.rs`; the vanished-file reporting shared by both protocols is `src-tauri/src/missing.rs`.

## Tests

- Core unit and integration tests as listed in steps 1–5.
- `src-tauri`: `folder-missing` from both protocols and from a refresh of a vanished root (temp dir renamed);  id and mark carry-over in `Scan::refresh`; `410` for a vanished file in both protocols; `photos-missing` once per revision (pure throttle helper); `reject_in` with a vanished file returns `PhotoMissing` and logs nothing; undo with a taken name.
- Vitest: reducer `refreshStarted` and the folder-missing state (set by `folder-missing`, cleared by `scan-refreshed`); the folder-missing notice and its button; the bar's „Odśwież” (enabled/disabled, F5); keymap uniqueness stays green; the pill note for missing photos; tile retries after a revision change; viewer missing message and its button.
- Playwright: „Odśwież” runs a refresh (mocked commands) and the pill shows „Sprawdzanie zmian…”; the viewer's missing message.

## Verification

- All checks and tests (CLAUDE.md), `npm run e2e`.
- Debug build, by the user: open a folder, rename a subfolder in Finder/Explorer, then (a) open a photo of it in the viewer: the missing message and „Odśwież folder”; (b) press F5: „Sprawdzanie zmian…” is short (nothing read again), the photos show at their new folder with their accepted marks, the viewer shows the photo again. Rename the opened folder itself and press F5: the folder-missing notice with „Wybierz inny folder”; choose it under its new name: it opens quickly (nothing read again) with its accepted marks. Reject a photo, put another file under its name, „Cofnij”: it comes back as `(2)`.
