# Slice 4 – Thumbnails

**Status:** 🧪 ready for testing (v0.3.0)

## Context

Slice 3 shows every photo of a folder on a virtualized timeline, but each tile is a placeholder with time and file name. Slice 4 puts the real picture into the tile, so the user can recognize the photos and, later, judge series (slice 5+) and compare shots (slice 7).

Requirements (ROADMAP): real thumbnails in the timeline, generated in the background, cached on disk, loaded lazily for the visible area only, correct EXIF rotation. Must stay smooth with 10,000+ photos on a 7 GB machine.

User decisions:
- **On demand:** thumbnails are generated only for tiles that are on screen (plus a small margin). No background pass over the whole folder.
- **Standard app cache dir:** Tauri's `app_cache_dir()`, e.g. `%LOCALAPPDATA%\io.github.lukaszkalnik.sito\thumbnails` on Windows or `~/Library/Caches/io.github.lukaszkalnik.sito/thumbnails` on macOS. No size limit in this slice.

## Design

### Core (`crates/core`)
- **`timeline.rs`:** add `modified: Option<SystemTime>` to `Photo`. `FoundFile` already has it. Together with path and size, it forms the cache key, the same key as in the CLAUDE.md storage plan.
- **New `thumbnail.rs`:**
  - `render_thumbnail(path, orientation, max_edge) -> Result<Vec<u8>, ThumbnailError>`. Steps:
    1. Decode **downscaled** with `jpeg-decoder`'s DCT scaling (`Decoder::scale`). This decodes a 12 MP photo at 1/8 size directly, about 10× faster than a full decode, and keeps memory low.
    2. Fall back to a full `image` decode if that fails (e.g. a CMYK or odd JPEG).
    3. Resize with `image` (Triangle filter) to fit inside `max_edge` × `max_edge` (default **400 px**: covers a 160×120 tile at 2× DPI). Never upscale.
    4. Apply EXIF orientation 1–8 (rotate and flip).
    5. Encode JPEG with quality 80, about 15–25 KB per thumbnail.
  - `ThumbnailCache { dir }` with `get_or_create(&Photo) -> Result<Vec<u8>, ThumbnailError>`:
    - file name = hex `blake3` hash of `(FORMAT_VERSION, path, size, mtime)`, sharded as `ab/abcd….jpg` so no folder holds 10,000+ files
    - when the file exists, read it; otherwise render it and write atomically: temp file in the same folder, then rename, so concurrent requests or a crash never leave half-written files
    - bumping `FORMAT_VERSION` invalidates old thumbnails
  - `ThumbnailError` (thiserror): `Io { path, source }` and `Decode { path, … }`.
  - New deps: `image` (jpeg feature only, from the tech stack), `jpeg-decoder`, `blake3`.
- Reusable later: the downscaled decode is what slice 10 (sharpness) needs too.

### Tauri (`src-tauri`)
- **Custom URI scheme `thumb`:** registered with `register_asynchronous_uri_scheme_protocol`.
  - URL: `thumb://localhost/<id>?scan=<n>` on macOS, `http://thumb.localhost/<id>?scan=<n>` on Windows, built in the UI with `convertFileSrc`.
  - The handler looks up photo `<id>` in `AppState`'s timeline and runs `ThumbnailCache::get_or_create` on a **dedicated rayon pool with 2 threads**, so it is memory-safe and leaves the UI thread free.
  - Response: `200 image/jpeg` with `Cache-Control: max-age` so the webview's memory cache serves re-scrolls. Errors: `404` for an unknown id, `500` with a logged reason when decoding fails.
  - `?scan=` is a cache-buster: ids are positions that change with every scan.
- **`AppState`** gains the `ThumbnailCache`, created in `setup` from `app.path().app_cache_dir()/thumbnails`. If that fails, it falls back to the temp dir and logs a warning.
- **CSP:** `img-src` adds `thumb: http://thumb.localhost`.
- Pure helpers get unit tests: parsing the `<id>` from the URI, and mapping a result to a status code and headers.

### UI (`ui`)
- `api/thumbnails.ts`: `thumbnailUrl(id, scanId)` wraps `convertFileSrc(String(id), 'thumb')` plus `?scan=`.
- **`TimelineTile` component**, extracted from `TimelineGrid`:
  - an `<img>` fills the tile (`object-fit: cover`, `loading="lazy"`, `decoding="async"`, empty `alt` because the name is in the tile)
  - time and name are overlaid at the bottom on a gradient; the "data pliku" badge stays
  - **lazy and scroll-aware:** the `<img>` is only mounted once the tile has been on screen for about 150 ms (`useDelayedVisible`, pure hook with fake-timer tests). Tiles that fly past during fast scrolling never request a thumbnail; virtualization already limits what's on screen.
  - **states:** a placeholder (current tile look) while loading, a fade-in on load, and on error the placeholder with a small "broken image" mark and the hint `timeline.thumbnailError` in pl.json
- `scanId` (already in `useTimelineEntries`) is passed down for the cache-buster.

### Tests
- **Core unit and integration tests** (`crates/core/tests/thumbnail.rs`). Test images are generated in a temp dir with `image`, e.g. 1600×1200 with a red top-left corner, so nothing large is committed. Covered:
  - size limit and aspect ratio kept
  - no upscaling of the 16×12 fixtures
  - all 8 orientations: dimensions swapped for 5–8, and the red corner ends up where expected
  - cache hit returns the same bytes without re-rendering (mtime of the cache file unchanged)
  - changed size or mtime gives a new key
  - a corrupt JPEG gives `Decode`, not a panic
  - concurrent `get_or_create` of the same photo is safe
- **Tauri unit tests** for URI parsing and response mapping.
- **Vitest:** `useDelayedVisible`; `TimelineTile` (img appears after the delay, the error state shows the hint, the badge is still shown).
- **Playwright:** the mock gets `convertFileSrc`, and `page.route('http://thumb.localhost/**')` serves a tiny PNG. Tests check:
  - thumbnails appear for visible tiles
  - after a fast scroll to the end, far fewer thumbnail requests than 10,000 (e.g. under 200)
  - a failing thumbnail shows the error state

## Implementation steps (one commit each, pushed)
0. `docs: add slice 4 plan`: save as `docs/slices/04-thumbnails.md`, ROADMAP slice 4 → 🚧.
1. `feat(core): render downscaled, rotated thumbnails` (`thumbnail.rs` render plus tests, `Photo.modified`).
2. `feat(core): cache thumbnails on disk` (`ThumbnailCache` plus tests).
3. `feat(tauri): serve thumbnails through a thumb:// protocol` (protocol, pool, state, CSP, tests).
4. `feat(ui): lazy thumbnails in timeline tiles` (tile component, hook, i18n, unit and e2e tests).
5. `chore: release v0.3.0` after checks and CI are green (the tag push needs your OK). ROADMAP → 🧪.

## Verification
- **Local**, one heavy job at a time: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`. Plus one `npm run tauri dev` run on a real folder, to look at speed and rotation myself.
- **CI** green on Windows and macOS; the v0.3.0 release workflow, including the smoke test, is green.
- **Manual** (you, with the installed v0.3.0) on your 10,000+ photo gallery:
  - thumbnails appear within about a second for the visible screen
  - fast scrolling stays smooth, and stopping fills the new screen
  - portrait photos are upright
  - reopening the same folder shows thumbnails instantly from the cache
  - the cache folder size is reasonable (about 20 KB per viewed photo)

## Risks / notes
- **Cache size:** the cache is on C: (about 200 MB per 10,000 viewed photos), with no size limit or cleanup yet; that's a later slice or setting. "Usuń dane programu" in the uninstaller removes it.
- **`jpeg-decoder`** is in maintenance mode but stable; it's only the fast path, with `image` as fallback. If its scaled decode turns out wrong or slow for Android JPEGs, switch to `zune-jpeg` and resize.
- **WebView2 and old requests:** WebView2 may not cancel in-flight custom-protocol requests for unmounted images. The 150 ms visibility delay plus the 2-thread pool bound the waste. If the queue still lags after fast scrolls, add a LIFO queue that drops requests for unmounted tiles.
- **Format:** HEIC/PNG are not scanned yet (JPEG only, as in slice 1).
