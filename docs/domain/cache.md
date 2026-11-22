# Cache

What Sito keeps besides the photos and the action log, how large it may grow, emptying it, and where it lives. Shipped with [slice 15](../slices/15-settings.md).

## What the cache holds

Everything in the cache can be rebuilt from the photos, so it may be emptied or moved at any time. The action log (decisions, accepted marks, series edits, settings) is not part of it: it stays in the app data folder.

- **`thumbnails/`**: thumbnails (400 px) and the viewer's JPEGs of TIFF photos (4096 px, [Photos](photos.md)), spread over 256 subfolders named by the first two hex digits of the file name.
- **`photos.db`**: what scans read from photos (capture times, cameras, pending EXIF of a Takeout's first pass, [Photos](photos.md#first-scan-of-a-takeout)). A few MB per 10,000 photos; it has no size limit.

Only these two are Sito's cache. The default cache folder on Windows also holds the webview's own data (`EBWebView`), which is never moved or removed.

## Settings: „Ustawienia → Pamięć podręczna…”

The menu item (en „Cache…”, de „Zwischenspeicher…“; on macOS in the app menu with the language) opens a dialog in the main window, titled „Pamięć podręczna”, closed with „Zamknij” or Escape (not while the cache is being moved).

### Size

- **„Miejsce na miniatury”**: how much `thumbnails/` may take, in MB, default 500. Any whole number from 50 up; a typed value applies when the field is left or Enter is pressed, and an empty or smaller value becomes 50. A smaller limit cleans the cache at once, in the background. After each scan and refresh, thumbnails of photos no longer known are removed and, above the limit, the oldest ones. The limit is saved in the action log's settings (`thumbnail_limit_mb`).
- **In use now:** „Miniatury: 412 MB · Dane zdjęć: 18 MB”, measured when the dialog opens (a spinner until then). Whole MB from 1 MB, below in whole kB, in the locale's number format.

### Emptying

- **„Wyczyść miniatury”** asks „Usunąć wszystkie miniatury z pamięci podręcznej? Zostaną utworzone ponownie przy przeglądaniu zdjęć.” (Usuń / Anuluj) and removes every cached thumbnail and display JPEG; only files named like cached thumbnails are touched. Thumbnails already shown stay on screen; others are rendered again as the grid needs them.
- **„Wyczyść dane zdjęć”** asks first and names the cost („Wszystkie foldery zostaną przy następnym otwarciu odczytane od nowa – przy dużych folderach może to potrwać kilka minut.”), then empties `photos.db`. Disabled (with „Dostępne po zakończeniu wczytywania folderu.”) while a folder is scanned, refreshed or in its EXIF pass. The open folder stays shown; its next opening or refresh reads it in full. Accepted marks, series edits and rejections are in the action log, so they stay.
- Emptying also serves privacy: thumbnails of photos stay in the cache until it is cleaned or emptied ([Security](security.md)).

### Where it is

- **Windows:** `%LOCALAPPDATA%\io.github.lukaszkalnik.sito\` by default, usually on a small C: drive. „Folder pamięci podręcznej” shows the folder in use (cut in the middle, full path as tooltip) with **„Zmień…”** and, away from the default, **„Przywróć domyślny”**.
- **macOS:** `~/Library/Caches/io.github.lukaszkalnik.sito/`, shown with **„Pokaż w Finderze”**; it cannot be moved (Macs usually have one drive, a cache on an external one would vanish when it is unplugged, and macOS frees `~/Library/Caches` itself when space runs low). Linux, which only runs the tests, shows „Pokaż w menedżerze plików”.

## Moving the cache (Windows)

- **The chosen folder gets a subfolder `Sito-cache`** (e.g. `D:\Cache\Sito-cache`) with an empty `.nomedia` file, so moving and emptying never touch anything else in the chosen folder.
- **Refused folders,** each with its own message: not writable; inside the folder open in Sito (its scans would walk the cache); the cache folder in use, a folder inside it or one holding it, or a folder inside the default cache folder; a symbolic link or junction (the folder itself or its `Sito-cache`); a name that cannot be stored as text. Paths are compared after resolving links and `..`.
- **A network drive** is allowed; the confirmation warns „Pamięć podręczna na dysku sieciowym może spowolnić Sito.”
- **Confirmation:** „Przenieść pamięć podręczną (412 MB) do D:\Cache\Sito-cache?” (Przenieś / Anuluj); „Przywróć domyślny” asks „Przenieść pamięć podręczną z powrotem do folderu domyślnego?”.
- **While a folder is scanned or refreshed** the cache does not move: „Zmień…” and „Przywróć domyślny” are disabled, and a move started meanwhile fails with „Trwa wczytywanie folderu. Spróbuj ponownie, gdy się zakończy.” The EXIF pass of a Takeout is stopped for the move and goes on afterwards from where it stopped. Thumbnail requests wait for the move and then render into the new folder; the grid stays as it is throughout.
- **On the same drive** the folder is renamed, which is instant. **On another drive** (or when the target already has a `thumbnails` folder) every file is copied and checked against its original (size and BLAKE3 hash), with „Przenoszenie: 1234 z 41 000 plików” and „Anuluj”; then Sito switches over and only then removes the old copy (Sito's own cache, not the user's files). A failure or „Anuluj” leaves the cache where it was and removes what was copied („Przenoszenie anulowane. Sito nadal używa dotychczasowego folderu.”, or the reason).
- **A `Sito-cache` that already holds a cache** (used before) is used as it is: moved files replace those of the same name, other cache files there stay and count toward the limit, and `photos.db` is replaced.
- **Moving back to the default** puts the files into the default folder itself, without a subfolder; the emptied `Sito-cache` is removed.
- **The choice** is saved in the action log's settings (`cache_dir`, empty for the default), so it survives emptying the cache.

## The chosen folder is gone on start

If the chosen folder is missing when Sito starts (drive unplugged, folder deleted), Sito uses the default folder for that session and a notice at the bottom left says „Folder pamięci podręcznej D:\Cache\Sito-cache jest niedostępny – Sito używa folderu domyślnego.” with „Ustawienia…” and „Zamknij”. The choice stays, so the next start with the drive back uses it again. In the dialog, „Przywróć domyślny” then only drops the choice (nothing is moved); „Zmień…” moves the default folder's cache to a new place. Nothing is copied back and forth.
