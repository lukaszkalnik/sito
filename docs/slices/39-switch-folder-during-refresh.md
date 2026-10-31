# Slice 39 – Opening a folder while another is still checked

**Status:** 📝 planned (plan approved 2026-10-31)

## Context

From the user on 2026-10-31, while testing slice 36: after renaming the year folders of the Takeout outside Sito, choosing another folder hangs at „Wyszukiwanie zdjęć… znaleziono 0 zdjęć”, in the debug build and in the installed release alike (they share the cache and the last folder).

**Cause** (from the code and the log):
- On start, Sito reopens the last folder from the photo cache and rescans it in the background (`refresh_in_background` in `src-tauri/src/scan.rs`, note „Sprawdzanie zmian…”).
- `PhotoCacheState::scan` (`src-tauri/src/scan/photo_cache.rs`) holds the cache mutex for the **whole** `build_timeline_cached` (`crates/core/src/timeline.rs`): the folder walk, reading every new photo, and storing them.
- After a rename every photo has a new path, so that refresh reads all ~34,000 photos from E:. A newly chosen folder's `start_scan` first calls `cached_timeline`, which waits for the same mutex until the refresh ends. Nothing stops the refresh when another folder is opened.
- What it read is only stored at the very end, so closing Sito mid-refresh throws the work away and the next start begins again.

**Outcome:** opening another folder is never held up by the background work of the previous one; a long refresh keeps what it has read.

## Plan

### Core (`crates/core`)

1. **Cache locked per step.** `build_timeline_cached(root, cache: &SharedCache, mode, should_stop, on_progress)` where `pub type SharedCache = Mutex<Option<PhotoCache>>` (in `photo_cache.rs`; `None` = no cache). It locks only to look up the cached photos of `root` (`split_cached`), to store each read chunk, and for the final `forget_missing`; never during the walk or while reading files. `build_timeline` passes an empty one. Poisoned lock: `PoisonError::into_inner` (the cache only speeds things up).
2. **Read chunks stored as they are read.** `read_photos` (`timeline/read.rs`) takes a `store` callback called after each chunk of `READ_CHUNK`, so a stopped or killed scan keeps what it read (a resumed refresh reads only the rest). `update_cache` then only does `forget_missing`, and only when the scan finished.
3. **Stoppable scans.** `should_stop: impl Fn() -> bool` is checked by `find_photos` (`scan.rs`) at each progress interval and by `read_photos` before each chunk; when it says so they return the new `ScanError::Stopped` (documented as "stopped on request, not a failure").
4. Update callers in `crates/core/tests/photo_cache.rs` and others (`Mutex::new(Some(cache))`, `|| false`).

### Tauri (`src-tauri`)

5. **Scan generation.** `AppState` gets `started: AtomicU64`: `begin_scan() -> u64` bumps it, `is_latest(generation)` compares. `start_scan` calls `begin_scan` first thing.
6. `PhotoCacheState::scan` takes `should_stop` and passes `&self.0` (the `SharedCache`) instead of holding the lock; the `ReadMode` choice reads `is_some()` under a short lock.
7. `refresh_in_background(app, path, scan_id, generation)` passes `|| !state.is_latest(generation)`; on `ScanError::Stopped` it returns silently (a newer folder replaced it). The foreground scan passes `|| false` (the UI blocks a second pick meanwhile); `Stopped` maps to `CommandError::Internal` defensively.
8. **Pass 2 stops too:** `complete_in_background` gets the generation and stops on `!is_current(scan_id) || !is_latest(generation)`, so a new folder's scan does not share the disk with the old folder's EXIF pass.

### Docs

9. This file: status 📝 → 🧪. `docs/domain/photos.md`: opening another folder stops the refresh and EXIF pass of the previous one; a refresh keeps what it read. [Slice 38](38-files-changed-outside.md): note it builds on the stoppable, chunk-storing refresh (its „Odśwież” button reuses it).

## Tests

- Core (`tests/photo_cache.rs`, temp dirs with copied fixtures):
  - the cache mutex is free while photos are read (`try_lock` succeeds inside the `Reading` progress callback);
  - a scan stopped after its first chunk returns `Stopped`, and the photos of that chunk are in the cache while the others are not (needs > `READ_CHUNK` files: copy one fixture 201 times);
  - a stopped scan forgets nothing (`forget_missing` not run);
  - `find_photos` with `should_stop` true returns `Stopped`.
- `src-tauri` unit test: `begin_scan` makes earlier generations not latest.
- Existing Rust tests and e2e stay green (no UI change).

## Verification

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run lint`, `npm test`, `npm run e2e`.
- Debug build, by the user: start Sito (it reopens „Zdjęcia Google” and refreshes), pick another folder right away: its scan starts at once. Close Sito mid-refresh and restart: the refresh goes on faster than before.
