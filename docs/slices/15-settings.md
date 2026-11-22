---
status: 📝 planned
---

# Slice 15 – Settings: cache location and size

Planned on 2026-11-22 in a cloud session (Linux container); the user decided that choosing the cache location is **Windows only** and answered the open questions the same day ([Decisions](#decisions)).

## Idea

A settings screen with the cache: how large it may grow, how much it uses now, emptying it, and on Windows where it lives (the existing cache moves over). It is also the place for later settings ([slice 17](17-sharpness.md)'s blur threshold, [slice 42](42-zero-click-update.md)'s automatic updates).

**Why the location is Windows only:** Sito's cache is in the system's app cache folder: on Windows `%LOCALAPPDATA%\io.github.lukaszkalnik.sito\`, always on C:, which is often a small system drive next to a large D:. On macOS it is `~/Library/Caches/io.github.lukaszkalnik.sito/`: Macs usually have one internal drive, a cache on an external drive would be gone whenever it is unplugged, and macOS itself frees `~/Library/Caches` when space runs low. Linux only runs the tests. So macOS shows where the cache is, and only the size limit and emptying.

The option to reject into the system trash moved to [slice 34](34-reject-to-trash.md) (a picker in the main window); the settings screen may show it later as well.

## What the cache holds today

All of it can be rebuilt from the photos, so it may be emptied or moved at any time (unlike the action log, which stays where it is, in the app data folder):

- `thumbnails/`: thumbnails (400 px) and the viewer's JPEGs of TIFF photos (4096 px). Limited to 500 MB, oldest removed first, cleaned after scans (`ThumbnailCache::clean`, `THUMBNAIL_CACHE_LIMIT` in `src-tauri/src/scan/photo_cache.rs`).
- `photos.db`: what scans read from photos (capture times, cameras; the analysis scores of slice 17 later). No limit; a few MB per 10,000 photos. Emptying it means every folder is read again in full on its next opening (minutes for a Takeout on a USB disk).

## Behaviour

### Opening the settings

- The menu gets **„Ustawienia → Pamięć podręczna…”** (en „Cache…”, de „Zwischenspeicher…“), below „Język”; on macOS in the app menu with the language, as today ([Languages](../domain/languages.md#which-language)). Language stays in the menu; the screen covers what does not fit into a menu.
- It opens a **dialog in the main window** (like „Licencje…”), titled „Pamięć podręczna”, closed with „Zamknij” or Escape. Keyboard-usable, strings in `pl.json`, `en.json`, `de.json`; menu labels in `labels.rs`.

### Size

- **„Miejsce na miniatury”** (thumbnail space): a number input in MB (default 500), any whole number from 50 up; an empty or smaller value is clamped to 50 when the field is left, like the gap input. It limits `thumbnails/` as today; a smaller value cleans the cache right away, in the background. A value above the drive's free space is allowed (the cache simply fills what there is).
- **In use now:** „Miniatury: 412 MB · Dane zdjęć: 18 MB” (file sizes in the locale's format, as in the viewer). Measured when the dialog opens, in the background (a spinner until then; a large cache folder takes a moment to sum up).

### Emptying

- **„Wyczyść miniatury”**: removes every cached thumbnail and display JPEG; they are rendered again as the grid needs them. Asks first: „Usunąć wszystkie miniatury z pamięci podręcznej? Zostaną utworzone ponownie przy przeglądaniu zdjęć.” (Usuń / Anuluj). Touches only files named like cached thumbnails, as the cleaning does today. Photos are never touched.
- **„Wyczyść dane zdjęć”**: empties `photos.db`. Asks first and says the cost: „Wszystkie foldery zostaną przy następnym otwarciu odczytane od nowa – przy dużych folderach może to potrwać kilka minut.” Disabled while a folder is scanned, refreshed or in its EXIF pass. The open folder stays shown; its next refresh reads it in full. Accepted marks, series edits and rejections are in the action log, so they stay.
- Emptying also serves privacy: thumbnails of photos stay in the cache until it is cleaned ([Security](../domain/security.md)).

### Location (Windows only)

- **„Folder pamięci podręcznej”** shows the current folder (cut in the middle like the main window's path, full path as tooltip), with **„Zmień…”** (a folder picker) and, when not the default, **„Przywróć domyślny”**. On macOS (and Linux) the folder is shown with **„Pokaż w Finderze”** (Linux: „Pokaż w menedżerze plików”), opened from the Rust side, and without „Zmień…”.
- **The chosen folder gets a subfolder `Sito-cache`** (e.g. `D:\Cache\Sito-cache`), so emptying and moving never touch anything else in the chosen folder. The subfolder gets a `.nomedia` file, so a scan of a folder above it never shows thumbnails as photos ([Photos](../domain/photos.md#the-galleries)).
- **Refused folders,** each with its own message: not writable; inside a folder that is open in Sito (a scan would walk it); the current cache folder or a folder inside it; a symbolic link or junction (like rejected folders, [Security](../domain/security.md)). A network drive is allowed with a warning in the confirmation („Pamięć podręczna na dysku sieciowym może spowolnić Sito.”).
- **Moving:** after the user confirms („Przenieść pamięć podręczną (412 MB) do D:\Cache\Sito-cache?”), Sito stops its background work (thumbnail rendering, the EXIF pass, refreshes, later the analysis pass of slice 17), closes `photos.db`, and moves the cache:
  - on the same drive by renaming the folder (instant);
  - to another drive by copying every file, checking each copy (size and BLAKE3 hash, as for moves of photos across drives), then switching over, and only then removing the old copy. The old copy is Sito's own cache, not the user's files, so it is removed (the „never delete” rule is about the gallery).
  - Progress in the dialog („Przenoszenie: 1234 z 41 000 plików”), cancellable until the switch. A failure or cancel leaves the old cache in use and removes the partial copy; the error is shown.
  - Then the cache reopens at the new place and background work goes on. The grid stays as it is throughout (thumbnails already shown stay; new ones wait for the move).
- **A `Sito-cache` that already holds a cache** (e.g. used before, then moved back to the default) is used as it is: the moved files overwrite files of the same name there, without a warning (it is all rebuildable cache), and other cache files there stay and count toward the limit. `photos.db` is replaced by the one being moved.
- **The setting** is kept in the action log's settings (key `cache_dir`), not in the cache itself, so it survives emptying.
- **The folder is gone on start** (drive D: removed, folder deleted): Sito uses the default folder for this session and the bar's pill says „Folder pamięci podręcznej D:\Cache\Sito-cache jest niedostępny – Sito używa folderu domyślnego.” The setting stays, so the next start with the drive back uses it again; the dialog offers „Przywróć domyślny”. Nothing is copied back and forth.

## Code

- `crates/core/src/cache_dir.rs` (+ `cache_dir/`): pure rules (is a folder allowed, the `Sito-cache` subfolder, which files belong to the cache), the move (rename or copy-verify-switch-remove, cancellable, with progress), and summing up the cache's size. Reuses the cross-drive copy and verification of `reject/` (moving photos across drives) where it fits; otherwise a shared helper is extracted in a `refactor:` commit first.
- `crates/core/src/thumbnail.rs`: `clean` takes the limit as a parameter already; add `clear` (remove all cached thumbnail files).
- `src-tauri/src/cache_settings.rs` (+ submodules): commands `get_cache_info` (folder, default or chosen, is it the fallback, sizes; Windows-only fields as a discriminated union per platform), `set_thumbnail_limit`, `clear_thumbnails`, `clear_photo_data`, `move_cache(folder)` with a progress channel and `cancel_cache_move`; the state that stops and resumes background work around a move (a lock the scans, the EXIF pass and the thumbnail workers take, like the reject/accept lock). `lib.rs` reads `cache_dir` before opening the caches.
- `src-tauri/src/menu.rs`: the new item and its action; `labels.rs` in three languages.
- `ui/src/features/settings/`: the dialog, a reducer for the move (`idle | confirming | moving { done, total } | failed { code }`), `api.ts` wrappers, error codes mapped to messages.
- `docs/domain/`: a new topic file `cache.md` (what the cache holds, limits, emptying, location), linked from the index; `timeline.md` and `security.md` point to it.

## Tests

Cases, listed before writing them:

- **Rules (unit):** allowed and refused folders (not writable, inside an open folder, equal to or inside the current cache, a symlink or junction, the default folder itself); the subfolder name.
- **Move (unit, temp dirs):** rename on one file system; copy-verify-switch on a forced copy path (as the cross-drive photo moves are tested); a failing copy (read-only target, a file that changes during the copy) leaves the old cache in use and no partial copy; cancel before the switch; progress counts; non-ASCII and long paths; an empty cache; the `.nomedia` file; a target `Sito-cache` that already holds a cache (same-named files overwritten, others kept, `photos.db` replaced).
- **Size and emptying (unit):** a lower limit removes the oldest thumbnails; `clear` removes only cache-named files; emptying `photos.db` leaves an empty, working cache; sizes summed correctly.
- **Start (unit/integration in `src-tauri`):** a chosen folder that exists is used; a missing one falls back to the default and reports it; the setting stays.
- **UI (Vitest):** the dialog in all three languages (file sizes per locale); the size input (typing, clamping to 50, applying); macOS with „Pokaż w Finderze” and without „Zmień…”; the network-drive warning; confirmations; the move's progress, cancel and errors; „Wyczyść dane zdjęć” disabled during a scan.
- **Mocked e2e (Playwright):** open the dialog from the menu event, change the limit, empty thumbnails (tiles load again), move the cache with progress, the fallback notice.
- **Real app (`e2e-app/`, one spec, Windows only, run in CI's Windows job):** move the cache to another folder while a gallery from `fixtures/` is open, check on disk that the cache is in the new folder and gone from the old one, restart, the folder opens from the moved cache without reading the photos again; restore the default.

## Testing without and with the user's machine

- **In the container:** everything above except the real-app spec, which only CI's Windows job runs; the move itself is covered by core tests on Linux (across file systems with a tmpfs mount if the container allows it, otherwise through the forced copy path).
- **On the user's machine (Windows):** move the cache from C: to D: with the Takeout's cache (several hundred MB, tens of thousands of files) and time it; check that Sito still starts instantly and the Takeout opens from the moved cache; unplug or rename the folder and start Sito (fallback notice); restore the default. On macOS later: the dialog without location.

## Decisions

Answered by the user on 2026-11-22:

1. **A dialog** in the main window, opened from „Ustawienia → Pamięć podręczna…”.
2. **Any number in MB** for the thumbnail space (from 50 up).
3. **macOS shows the cache folder** with a button that opens it in Finder.
4. **Network drives are allowed,** with a warning.
5. **A folder that already holds a Sito cache is used as it is;** moved files overwrite same-named ones without a warning.
6. **The real-app test of the move runs on Windows only.**

## Related

- [Slice 17](17-sharpness.md): the blur threshold could get a field here (planned there as a hidden setting until this slice exists); its analysis scores live in `photos.db` and are emptied with it.
- [Slice 42](42-zero-click-update.md): „Aktualizuj automatycznie” could live on this screen instead of the Pomoc menu.
- [Slice 34](34-reject-to-trash.md): the rejection target could be shown here too.

## Try

Move the cache to another drive and check Sito still starts instantly.
