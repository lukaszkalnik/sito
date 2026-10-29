# Slice 27a – Fixes from testing slice 27 (TIFF)

**Status:** 🧪 ready for testing (what it established is in [docs/domain/photos.md](../domain/photos.md))

## Context

Found by the user on 2026-10-29 while testing [slice 27](27-tiff-photos.md) with an 8000×6000 test TIFF (`D:\Lukasz\tmp\sito-tiff-test`):

- The viewer showed only a black area for seconds while the TIFF was converted, with no sign that anything was happening.
- The full-size viewer JPEGs of real scans would be ~10–18 MB each and push thumbnails out of the shared 500 MB cache. The user chose to cap them at 4096 px (2026-10-29).

# Plan (approved 2026-10-29)

## 1 Loading indicator in the viewer
The test TIFF showed only a black area for seconds while it was converted.
- `FullPhoto` (`ui/src/features/viewer/PhotoViewer.tsx`): while the current photo is neither loaded nor failed, a spinner centred over the photo area, after a 300 ms delay (fast photos never flash it); it also overlays the previous photo kept on screen. `role="status"`, label „Wczytywanie zdjęcia…” (`pl.json` `viewer.loading`). Same look as the boot spinner in `ui/index.html` (teal ring), styled in `PhotoViewer.css`.
- Tests: Vitest with fake timers (none before 300 ms, shown after, gone on load and on error); e2e: a delayed `photo://` route shows the spinner, then the photo.
- Commit `fix: show that the viewer is loading a slow photo`.

## 2 Viewer JPEGs of TIFFs at most 4096 px (user decision 2026-10-29)
Full-size JPEGs of real scans would be ~10–18 MB each and push thumbnails out of the shared 500 MB cache.
- `ThumbnailCache::display_jpeg_for` (`crates/core/src/thumbnail.rs`): resize to fit 4096 × 4096 (`FilterType::Lanczos3`, never upscale, `decode::fit_inside`), quality 90, orientation as now. The key's size marker becomes `DISPLAY_MAX_EDGE = 4096` instead of `u32::MAX`, so full-size files from the test build are cleaned as orphans.
- Tests: a 6000×3000 TIFF gives a 4096×2048 viewer JPEG; small TIFFs keep their size.
- `docs/domain/photos.md`: viewer JPEGs of TIFFs are at most 4096 px; slice 13's 100 % zoom shows that resolution for larger scans.
- Commit `fix: keep viewer JPEGs of TIFFs at most 4096 px`.

## 3 Done
Slice 27a 🧪 in both files; debug build, started only when the user says no scan is running. Slice 27 stays 🧪 until the user has tested both.

## Progress

- ✅ 1. Loading indicator in the viewer (2026-10-29): a teal spinner over the photo area after 300 ms while a photo loads, gone on load or error; Vitest and e2e tests.
- ✅ 2. Viewer JPEGs of TIFFs at most 4096 px (2026-10-29): resized with Lanczos3, cache key marker 4096 so full-size files from the test build are cleaned as orphans; test with a 6000×3000 TIFF.
- ✅ 3. Ready for testing (2026-10-29): debug build made; started when the user says no scan is running.
