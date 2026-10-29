# Slice 29 – Faster first scan of Takeout folders

**Status:** 🚧 in progress

## Context

The first scan of the user's unpacked Google Takeout (34,600 photos on a 5 TB WD My Passport USB hard disk) reads dates at about 1,000 photos per 40 s, so a full first scan takes 35–40 minutes. Later scans are fast because results are cached ([slice 9](09-persistence.md)).

Measured on 2026-10-29 (read-only timing tool on `F:\Google Photos`, fresh photos per run so the file cache does not help):

| What | Time per photo |
|---|---|
| Opening a photo file (no reading) | 30–50 ms |
| Opening and reading the whole photo (1.2 MB) | about the same |
| `nom-exif` reading the EXIF, as Sito does | 30–56 ms |
| Opening and reading its Takeout date file | 10–12 ms |
| The same on the internal NVMe SSD | under 1 ms |
| 1, 2, 4 or all-core parallel reads | no real difference (62–86 ms in total) |
| Finding the files (folder walk, 37k entries) | 34 s |

So the cost is the drive's random access per opened file (a mechanical seek over USB), not the amount of data or Sito's code. The antivirus is off for that folder. The small JSON date files are cheaper to open, probably because NTFS keeps files that small inside its file table.

## Concept

- **Takeout fast path:** for photos with a Takeout date file, the first pass takes the date from the date file only, without opening the photo. The timeline is shown after roughly a fifth of today's time.
- **EXIF later, in the background:** a second pass reads each photo's EXIF (orientation, camera, the time zone comparison of [slice 25](25-takeout-dates.md)) in walk order and updates dates and series through the existing refresh events (`scan-refreshed`, `timeline-changed`).
- **Until EXIF is read,** a photo's time is the Takeout date in the computer's time zone; photos taken abroad may sit a few hours off and then move. Thumbnails need the photo anyway and can read orientation then.
- **Only on slow first scans:** with a cache or on a fast drive, nothing changes.

## Open questions

- Is the time zone shift until the second pass acceptable, or should series wait for the EXIF pass?
- Should the first pass also skip date files and use only the walk (file dates), for galleries without Takeout?
- How is the background pass shown (status line, progress)?

## Verification (outline)

- First scan of `F:\Google Photos`: the timeline appears in a fraction of today's time; after the background pass, dates and series match today's result.
- The timing tool (2026-10-29 measurements) run again for comparison.

# Plan (approved 2026-10-29)

## Context
A first scan of the user's Takeout (34,600 photos on a USB hard disk, `F:`) takes 35–40 minutes: each photo file costs 30–50 ms just to open, its Takeout date file ~11 ms (measurements in `docs/slices/29-fast-takeout-scan.md`). Parallel reads don't help. The first pass dates photos from their date files without opening them, shows the timeline, and reads EXIF in the background.

User decisions (2026-10-29):
- **Show early:** timeline and series appear after the first pass; photos taken abroad may shift by hours when their EXIF is read.
- **Takeout only:** photos without a date file are read fully in the first pass, as today.
- **Status line** shows the background pass: „Uzupełnianie dat z EXIF: 1 234 z 34 600”; the timeline stays usable.

## Design
- **Pass 1 (blocking, with today's progress):** an uncached photo *with* a Takeout date file is dated from the date file alone (`choose_time(None, …)` = `Takeout(MissingExif)`, computer's time zone) and marked *EXIF pending*. Other photos are read fully.
- **Pass 2 (background):** reads EXIF of pending photos in walk order, in chunks of 200, storing each chunk in the cache and updating the timeline at most every ~10 s. Stops as soon as another folder is opened; pending photos stay pending in the cache and resume next time.
- **Warm start** (folder in cache): quick show + refresh as today (the refresh is pass 1 for changed files), then pass 2 for whatever is still pending.

## 0. Docs
Save Part C into `docs/slices/29-fast-takeout-scan.md` (concept kept above), 🚧 there and in `docs/ROADMAP.md`. Commit `docs: add slice 29 plan`.

## 1. Core: "EXIF pending" photos (`feat:`)
- `crates/core/src/timeline.rs`: `Photo.orientation` + `camera` become `Photo.exif: ExifInfo` with `enum ExifInfo { Pending, Read { orientation: Orientation, camera: Option<String> } }` (no boolean flag); helper `ExifInfo::orientation() -> Option<Orientation>`.
- `to_photo` (full, today) plus `to_photo_takeout_only(file)` (date file only, `ExifInfo::Pending`; falls back to `to_photo` when the date file is unreadable).
- `build_timeline_cached(root, cache, mode, on_progress)` with `ReadMode { Full, TakeoutFirst }`; `TakeoutFirst` uses `to_photo_takeout_only` for uncached files with a sidecar. `read_photo` (restore) stays full.
- `complete_pending(root, store, on_chunk, should_stop)`: reads the `Pending` photos of `root` in path order (rayon per chunk), stores each chunk, reports `(done, total)`, returns early when `should_stop()`. The cache lock is taken per chunk.
- Photo cache: `CACHE_VERSION` 3 → 4; `orientation` nullable, `exif_read INTEGER NOT NULL`; `pending_under(root)`.
- Tests: the Takeout fixture album in `TakeoutFirst` mode gives Takeout dates and `Pending` for photos with date files, full reads for the rest; `complete_pending` turns them into the same result as a full scan (`tests/takeout.rs` expectations); resumes after a stop.

## 2. Thumbnails and viewer with unknown orientation
- `thumbnail::render_photo` and `display::display_jpeg` take `Option<Orientation>`; `None` (pending) reads the orientation from the file being rendered anyway, so no thumbnail is cached sideways. JPEG viewing needs nothing (the webview reads EXIF).
- Test: a pending photo with orientation 6 gets an upright thumbnail.

## 3. Tauri: pass 2 in the background (`feat:`)
- `src-tauri/src/scan.rs`: `start_scan` uses `ReadMode::TakeoutFirst`; after a cold scan or the warm refresh, `complete_in_background(app, path, scan_id)` runs `complete_pending` on a blocking thread; `should_stop` = the current scan id differs.
- Events: `scan-completing { scanId, done, total }` per chunk; at most every ~10 s an `AppState::refresh` with the cached timeline and `timeline-changed`; at the end a final refresh, `scan-refreshed`, and `scan-completing` with `done == total`.
- `PhotoCacheState` (`scan/photo_cache.rs`): helpers that lock per call.

## 4. UI (`feat:`)
- `ui/src/features/scan/api.ts`: `onScanCompleting`; reducer action `completing` accepted in `done` (and `picking.previous`) for the matching scan id; state keeps `{ done, total } | null`.
- `TimelineView.tsx` `ScanStatus`: „Uzupełnianie dat z EXIF: {{done, number}} z {{total, number}}” while it runs (`pl.json` `scan.completing`).
- E2e mock emits `scan-completing`; Vitest for reducer and status line; one e2e test that the line appears and disappears.

## 5. Docs and check
- `docs/domain/photos.md`: two-pass first scan and what can move afterwards. Slice 🧪.
- Timing tool again; debug build on `F:\Google Photos` (read-only, when the user says so).

## Verification
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e` for each part.
- 29: `TakeoutFirst` + `complete_pending` equals a full scan on fixtures; manual first scan of the Takeout on F:; reopening mid-pass resumes; opening another folder stops pass 2.

## Progress

- ⬜ 1. Core: EXIF-pending photos and pass 2
- ⬜ 2. Thumbnails and viewer with unknown orientation
- ⬜ 3. Tauri: pass 2 in the background
- ⬜ 4. UI: status line
- ⬜ 5. Docs and check
