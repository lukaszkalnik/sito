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

**Cause:** the thumbnail pool ran the newest request first, so within one burst it rendered the rows below the screen first and the visible first page last.

**Fix:** `BurstQueue` (`src-tauri/src/thumbnails/burst_queue.rs`, unit tests there) serves the newest burst first and each burst in the order it came. A burst is a run of requests less than 100 ms apart, reaching back at most 300 ms from its newest request so a long continuous scroll does not chain back to the photos it scrolled past. The pool is now `BurstPool` (`thumbnails/burst_pool.rs`, formerly `lifo_pool.rs`).

**Checked (2026-10-29, debug build):** opening `F:\Google Photos\Takeout\Zdjęcia Google\Kreta 2019` (thumbnails not cached): the page filled top down; the 43 requests (one burst of 97 ms) were rendered in the order they were sent, neighbours swapped only by the two parallel workers.

## Note: the installed app resets the cache

Starting the installed v0.12.0 while testing (2026-10-29) emptied the photo cache: it uses cache version 3, this slice version 4, and each version throws away a cache of the other. The EXIF read so far is then read again. Until slice 29 is released, test with the debug build only.

## Testing (next)

All four fixes are in the debug build `target\debug\sito.exe` (start it with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` to log the webview's thumbnail requests with the CDP logger used for item 4). Still for the user to check: items 2 and 3 in the app, the EXIF pass finishing on `F:\Google Photos` with dates and series matching a full scan, and the user's OK for slices 29 and 29a before the release.
