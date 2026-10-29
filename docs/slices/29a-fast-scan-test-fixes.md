# Slice 29a – Fixes from testing slice 29

**Status:** 🚧 in progress

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
