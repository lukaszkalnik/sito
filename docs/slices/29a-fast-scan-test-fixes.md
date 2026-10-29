# Slice 29a – Fixes from testing slice 29

**Status:** 🧪 ready for testing

Problems found while testing [slice 29](29-fast-takeout-scan.md) on `F:\Google Photos` (34,600 photos, USB hard disk).

## 1. Thumbnails blocked during the background EXIF pass ✅

**Found (2026-10-29):** after pass 1 the timeline appeared (dates read at 10 s per 1,000 photos instead of 40 s), but while pass 2 ran, thumbnails were completely blocked: two appeared in about a minute.

**Cause:** `complete_pending` read each chunk of 200 photos with `rayon` on the global pool. Thumbnails decode with `jpeg-decoder`, whose `rayon` feature runs on that same pool, so every thumbnail decode queued behind whole chunks of disk reads (about 8 s each on that disk).

**Fix:** `complete_pending` reads one photo at a time on its own thread and never uses the rayon pool. The 2026-10-29 measurements showed parallel reads are no faster on that disk. Regression test: `completing_does_not_wait_for_the_busy_rayon_pool` in `crates/core/tests/takeout.rs` (fails on the old code with a timeout).

**Check:** restart the debug build. Pass 2 resumes with the photos still pending, and thumbnails should load while it runs (somewhat slower than without it, as both use the same disk). Confirmed by the user on 2026-10-29: thumbnails are "much faster".

## 2. First page of thumbnails stalls after opening another folder ✅ (fix to check)

**Found (2026-10-29):** switching from the Takeout (F:, background pass running) to `D:\Pulpit\Złote Gody` (683 JPEGs on the internal SSD), only two thumbnails of the first page appeared; tiles further down loaded when scrolling, and the first page loaded after a while.

**Cause:** the two thumbnail workers still had a queue of F: thumbnails requested in the Takeout. They rendered all of them (slow on that disk) although nothing showed them any more. The workers take the newest request first, so tiles scrolled to later were served before the first page, which waited for the old queue.

**Fix:** a queued thumbnail job first checks that its scan is still the one shown (`AppState::is_current`) and otherwise answers `410 Gone` without rendering (`thumbnail_response` in `src-tauri/src/thumbnails.rs`, unit test there).

**Check:** open the Takeout, scroll a bit, open another folder: its first page loads at once.

## 3. A resumed EXIF pass seems to start over ✅ (fix to check)

**Found (2026-10-29):** opening `F:\Google Photos` (which contains the Takeout folder scanned before) showed „Sprawdzanie zmian…” for about 37 s, then „Uzupełnianie dat z EXIF” counted from 0 of 26,400, although 8,200 of the 34,600 photos had been read before. The line also only appeared after the first chunk of 200 photos.

**Fix:** the progress counts all photos of the folder: photos whose EXIF was read before count as done (`PhotoCacheState::pending` returns them with the pending ones; `progress` in `scan/completion.rs`, unit test there). A first `scan-completing` with the starting count is sent before the first chunk, so the line appears at once.

**Not changed:** the warm refresh („Sprawdzanie zmian…”) still shows no progress; on that disk the folder walk alone takes about 35 s.

**Check:** restart the app on `F:\Google Photos`: after „Sprawdzanie zmian…”, the line appears at once and continues from the count it had.

## 4. The first page of thumbnails is rendered last ✅ (fix to check)

**Found (2026-10-29):** after item 2, opening `D:\Pulpit\Złote Gody` still showed two thumbnails of the first page and then nothing for a while; tiles further down loaded first.

**Measured** (network log of the webview over the Chrome DevTools Protocol, debug app started with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`): nothing hangs, the backend answers every request. A folder opened or a scroll position reached requests its thumbnails top to bottom in one burst (30 requests in 85 ms on opening, 72 within 15 ms after dragging the scrollbar; rows 20–50 ms apart), visible rows first, then the 4 rows rendered ahead below the screen. Separate scroll steps came 360–400 ms apart. Uncached thumbnails took about 0.6 s each on the SSD and about 1 s on the USB disk during the EXIF pass, with 2 workers.

**Cause:** every rendered tile requested its thumbnail 150 ms after mounting, visible or not, and the thumbnail pool ran the newest request first. Within one render it so rendered the rows rendered ahead below the screen first and the visible page last; after a jump into the middle, the rows rendered ahead above the screen were part of the same render too.

**Fix (as the user suggested):** the UI decides what loads. Tiles take turns in a queue (`ui/src/features/timeline/thumbnailQueue.ts`, unit tests there; `useThumbnailTurn` connects a tile): at most 3 load at once (one more than the 2 render workers); the visible tiles go first, top to bottom; while nothing visible waits, the tiles rendered ahead off screen, nearest first. Ranks are taken from the tiles' positions at the moment a turn is handed out. A tile gets a turn only after waiting 150 ms (so tiles flying past never load, and all tiles of one render compete together); one that unmounts or gets a new URL leaves the queue. Loading ahead stops where the grid stops rendering (4 rows above and below the screen). The backend pool (`thumbnails/worker_pool.rs`, formerly `lifo_pool.rs`) now keeps the order it gets. A first attempt timing bursts of requests in the backend (`6e32784`) was reverted. E2e test: `loads the visible thumbnails top to bottom before the rows rendered ahead` in `ui/e2e/thumbnails.spec.ts`.

**Checked (2026-10-29, debug build, after item 5):** in `Złote Gody`, dragging the scrollbar near the bottom (thumbnails not cached): the first thumbnail after 0.3 s, 20 within 3.4 s, the partly visible top row first, then the visible rows top to bottom, then the rows rendered ahead above, nearest first; all 56 requests done.

## 5. Thumbnails ten times slower in the debug build ✅

**Found (2026-10-29)** while checking item 4: each uncached thumbnail of `Złote Gody` (10 MP JPEGs) took about 2.2 s, so the screen filled at about one tile per second.

**Cause:** timing one photo: the downscaled `jpeg-decoder` decode took 2.3 s in a debug build and 0.18 s in a release build (a full decode with the `image` crate 0.24 s in both). `jpeg-decoder` is optimized in dev builds already, but its decoder is generic over the reader, so the decoding code is compiled inside `sito-core`, which was not.

**Fix:** `[profile.dev.package.sito-core] opt-level = 3` in `Cargo.toml`: the debug build now takes 0.22 s, like the release. The installed app was never affected.

## 6. The EXIF progress line hardly moves ✅ (fix to check)

**Found (2026-10-29):** „Uzupełnianie dat z EXIF” only changed once per chunk of 200 photos, 8 s or more on the USB disk.

**Fix:** `complete_pending` reports `(done, total)` after every photo (still storing in chunks of 200; test `completing_reports_progress_after_every_photo`); `scan/completion.rs` sends it to the UI at most once a second (`PROGRESS_EVERY`) and always for the last photo.

**Not a bug:** folders without Takeout date files (e.g. `D:\Pulpit\Złote Gody`) have no background pass and so no such line: their EXIF is read in the first scan.

## Note: the installed app resets the cache

Starting the installed v0.12.0 while testing (2026-10-29) emptied the photo cache: it uses cache version 3, this slice version 4, and each version throws away a cache of the other. The EXIF read so far is then read again. Until slice 29 is released, test with the debug build only.

## Testing (next)

All six fixes are in the debug build `target\debug\sito.exe` (start it with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` to log the webview's thumbnail requests with the CDP logger used for item 4). Still for the user to check: items 2, 3 and 4 in the app (fast scrolling in `Złote Gody`), the EXIF pass finishing on `F:\Google Photos` with dates and series matching a full scan, and the user's OK for slices 29 and 29a before the release.
