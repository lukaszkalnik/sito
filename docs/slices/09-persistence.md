# Slice 9 – Persistence

**Status:** ✅ done (tested by the user on 2026-10-27 with a debug build; released as v0.8.0)

## Context

Every start of Sito currently begins from scratch: pick the folder, then wait while it reads the EXIF data of every photo (tens of seconds for 10,000+ photos). Thumbnails are cached, but the cache only ever grows: thumbnails of edited, moved or deleted photos stay forever. C: is nearly full.

Slice 9 makes Sito remember its work. Rejections and undo already persist (slice 8), and so do window sizes.

Your decisions:
- **Reopen the last folder on start.** Its timeline appears at once from the cache, then Sito quietly checks the folder for new, changed or removed photos and updates the view.
- **The thumbnail cache is limited to 500 MB.** The oldest thumbnails go first. Thumbnails of photos that no longer exist are always removed.
- **The cache stays in the standard app folders on C:** (`%LOCALAPPDATA%\…`), and is kept small by the limit.

## Design

### Core (`crates/core`)
- **`photo_cache.rs`** (new): a `PhotoCache` on SQLite in the app cache folder (`photos.db`).
  - It is separate from the action log in `sito.db`, because it can always be rebuilt, while the log must never be lost.
  - Table `photos(path TEXT PRIMARY KEY, size INT, mtime_ns INT, taken_at TEXT, time_source INT, orientation INT, camera TEXT, seen_at TEXT)`.
  - `photos_under(root)` returns the cached photos of a folder tree, for the instant start.
  - `lookup(path, size, mtime)` returns a hit only if size and mtime are unchanged.
  - `store(&[Photo])` writes in one transaction.
  - `forget_missing(root, &found)` removes rows under `root` that a scan no longer found.
  - `all_keys()` lists every cached photo with its size and mtime, so the thumbnail cleanup knows which thumbnails are still needed.
- **`timeline.rs`:**
  - `build_timeline(root, cache: Option<&PhotoCache>, on_progress)` walks the folder as now.
  - Photos whose size and mtime are unchanged come from the cache; only new or changed ones are read, in parallel.
  - New readings are stored, and vanished photos are forgotten.
  - "Reading" progress counts only the photos that really need reading, so a rescan is near-instant.
  - `cached_timeline(root, cache)` builds a timeline from the cache alone (no file access) for the instant start, splitting rejected photos (`Odrzucone`) as the scanner does.
- **`thumbnail.rs`:** `ThumbnailCache::clean(needed, max_bytes) -> CleanReport`:
  - deletes thumbnail files that no cached photo needs (orphans)
  - then, while over the limit, deletes the oldest by file time
  - `needed` is computed with the existing `path_for` key from the cache's keys
  - it touches only `.jpg` files inside the thumbnail folder, never anything else
- **Tests:**
  - a rescan reads 0 files, and a changed file is read again
  - deleted photos are forgotten
  - `cached_timeline` matches the real scan
  - the cleanup removes orphans, keeps needed thumbnails, enforces the limit oldest-first, and ignores foreign files
  - corrupt or missing database files: the cache is rebuilt and nothing fails

### Tauri (`src-tauri`)
- **Setup:**
  - opens `photos.db` in `app_cache_dir()`; if it is unusable, it is deleted and recreated, since it is only a cache
  - after each scan, cleans the thumbnail cache on a background thread (limit 500 MB, `const` for now; slice 14 makes the limit and the cache location settings)
- **Stable scan across refresh:** `Scan::refresh(new_timeline)` keeps the scan id and the ids of photos that are still there, matched by path. It gives new photos fresh ids and drops vanished ones. Thumbnail URLs and the UI's references therefore stay valid, with no grey flashes.
- **`open_folder(path)` command,** used for the start and for picking a folder:
  1. If the cache knows photos under `path`, it sets the scan from `cached_timeline` and returns the summary at once, with `refreshing: true`.
  2. It then runs the real scan with the cache on a blocking thread, calls `Scan::refresh`, and emits `timeline-changed` and `scan-refreshed { summary }`.
  3. Without cached photos it scans with progress, as `start_scan` does today (`start_scan` stays as the no-cache path).
- **Tests:** `Scan::refresh` keeps ids and the scan id, adds fresh ids for new photos, and drops removed ones.

### UI (`ui`)
- **Last folder:** remembered in `localStorage` (`sito.lastFolder`, the same pattern as the gap setting). On start, `useScan` opens it automatically; if that fails (folder gone), it shows the start screen as now.
- **Refresh hint:** while `refreshing` is true, the status line ends with „Sprawdzanie zmian…". On `scan-refreshed`, the summary updates (photo count, without EXIF date, unreadable) and the hint disappears. The timeline itself reloads through the existing `timeline-changed` handling, and the scroll anchoring keeps the view in place.
- **Strings:** `scan.refreshing` "Sprawdzanie zmian…".
- **Tests:**
  - Vitest: the last folder is opened on start, and the summary updates on `scan-refreshed`
  - Playwright: the mock's `open_folder` returns a cached summary with `refreshing`, then emits the refresh; the spec checks that the timeline shows immediately, the hint, and the update

### Docs
- `docs/slices/09-persistence.md`; ROADMAP 9 → 🚧 → 🧪.

## Steps (one commit each, pushed)
0. `docs: plan slice 9`
1. `feat(core): cache photo metadata in SQLite`: cache, timeline with cache, `cached_timeline`, tests
2. `feat(core): clean up the thumbnail cache`: orphans, 500 MB limit, tests
3. `feat(tauri): open folders from the cache and refresh in the background`: `photos.db`, `open_folder`, `Scan::refresh`, events, cleanup after scans, tests
4. `feat(ui): reopen the last folder and show background refreshes`
5. A debug build for you to test. When you confirm: release v0.8.0 (installed silently), then slice 9 → ✅

## Verification
- **Local**, one heavy job at a time: `cargo fmt/clippy/test`, `npm run lint`, `typecheck`, `npm test`, `npm run e2e`.
- **Debug build** on a copy of a few hundred fixture photos in the scratchpad:
  - time the first and second scan, then restart and check the instant start
  - add, delete and change a photo; the refresh shows it
  - fill the thumbnail cache past a small test limit and check the cleanup
- **You**, with the debug build on your gallery:
  - the start reopens the last folder immediately, and „Sprawdzanie zmian…" disappears after a few seconds
  - picking a folder seen before is fast
  - the cache folder stays under 500 MB

## Risks / notes
- **Network or removable drives:** if the last folder is unreachable at start, Sito shows the start screen with the usual „Nie znaleziono wybranego folderu." message.
- **Time-zone or EXIF reader changes:** a `CACHE_VERSION` stored in the database invalidates all rows when the reading logic changes.
- **Undo after refresh:** ids of restored photos still come from `Scan::restore`, which works unchanged on a refreshed scan.

## Follow-up during testing
- After a fast scroll the thumbnails where the scroll stopped took 10 s or more: the two render workers served requests in arrival order. They now take the newest request first (`src-tauri/src/lifo_pool.rs`); thumbnails scrolled past are still rendered and cached afterwards. Real cancelling is not possible, since Tauri does not report requests the webview dropped.
- Debug builds (used for testing during a slice) now optimize the image libraries, so their thumbnails are as fast as in a release.
