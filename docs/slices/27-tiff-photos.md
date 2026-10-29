# Slice 27 – TIFF photos

**Status:** ✅ done (tested by the user on 2026-10-29; released as v0.12.0; what it established is in [docs/domain/photos.md](../domain/photos.md))

## Context

The user's Google Takeout (see [slice 25](25-takeout-dates.md)) contains TIFF files: about 60 in one zip, e.g. scans and reproductions. Sito only reads JPEG so far.

## Concept

- **Formats:** `.tif` and `.tiff`, read with the `image` crate's TIFF feature (already a dependency, so only a feature flag).
- **Capture time:** from TIFF/EXIF tags (`DateTimeOriginal`, else `DateTime`). Scans often have none and then fall back to the Takeout date (slice 25) or the file date.
- **Thumbnails:** decoded and downscaled like JPEG. TIFFs can be very large (100+ MB scans), so they're decoded with a memory cap. Pages beyond the first are ignored.
- **Viewer:** WebView2 can't show TIFF, so the backend serves a full-size JPEG rendering through the `photo://` protocol, cached like thumbnails.
- **16-bit and CMYK TIFFs** are converted to 8-bit RGB.

## Builds on slice 26

[Slice 26](26-raw-photos.md) built the pieces this slice extends, so it mostly adds a TIFF arm to each:

- `PhotoKind` and `photo_kind` (`crates/core/src/scan.rs`): add a `Tiff` kind for `.tif`/`.tiff`.
- The TIFF reader (`crates/core/src/tiff/`): IFD walker and `read_tiff_metadata` (`DateTimeOriginal`, else `DateTimeDigitized`, else `DateTime`; orientation; camera) already read TIFF files; dispatch `Tiff` to it in `metadata::read_photo_metadata`.
- Thumbnails: `thumbnail::render_photo` dispatches by kind and `ImageSource` decodes from a path or bytes; enable the `image` crate's `tiff` feature and add a memory cap.
- The viewer: `display::display_jpeg` dispatches by kind; a TIFF needs a rendered JPEG (plus a display cache), and its errors map to the existing „cannot load” path.
- Rejecting moves every file of a photo (`reject.rs`, `action_moves`), so nothing changes there.

## Open questions

- Is a full-size JPEG rendering cached on disk (fast, but takes space), or rendered on demand?
- Multi-page TIFFs: show only the first page (proposed)?

## Verification (outline)

- The TIFFs from the user's Takeout appear with thumbnails and open in the viewer.
- A very large scan doesn't run Sito out of memory.

# Plan (approved 2026-10-28)

## Decisions and context

The user's Google Takeout has TIFF files (about 60 in one zip: scans, reproductions), and Sito only reads JPEG and RAW. Slice 26 already built most of what TIFF needs (`docs/slices/27-tiff-photos.md`, section "Builds on slice 26"), so this slice adds a TIFF arm to each shared piece.

User decisions (2026-10-28):
- **Viewer JPEG cached on disk:** a TIFF is converted once to a full-resolution JPEG (so 100 % zoom in slice 13 shows real pixels), stored next to the thumbnails and counted in the same cache limit (500 MB, a setting in slice 15).
- **Multi-page TIFFs:** only the first page is shown; the file is one photo and moves as a whole.

Related later slices considered (CLAUDE.md rule): slice 13 (zoom: full-resolution viewer image), slice 15 (cache location and size: the display JPEGs live in the thumbnail cache, so moving/limiting it covers them), slices 17–18 (sharpness/exposure: they need every photo decoded to pixels the same way, so decoding by kind becomes one shared function they can call).

## Implementation

### 0. Docs
Save this plan into `docs/slices/27-tiff-photos.md` (concept and the slice-26 note kept above), status 🚧 there and in `docs/ROADMAP.md`. Commit `docs: add slice 27 plan`.

### 1. Shared decoding by kind (`refactor:`)
- New `crates/core/src/decode.rs`: `decode_photo(path, kind, max_edge: Option<u32>) -> Result<DynamicImage, DecodeError>`: JPEG via the fast scaled path / full decode, RAW via its preview, TIFF (step 2). Moves `decode_scaled`/`decode_full`/`ImageSource` out of `thumbnail.rs`, which then only resizes, orients and encodes. Slices 17–18 will analyse the result of the same function.
- No behaviour change; existing thumbnail and RAW tests stay green.

### 2. TIFF as a photo kind (`feat:`)
- `crates/core/Cargo.toml`: `image` gains the `tiff` feature. Check the licenses it pulls in (`tiff`, `weezl`, `fax`, `zune-*`, `flate2`/`miniz_oxide`) against `about.toml`; install `cargo-about` (`cargo install cargo-about --features cli`, under `D:\Lukasz\tools`) and run `npm run notices:rust`, commit `ui/notices/rust-crates.txt`.
- `scan.rs`: `PhotoKind::Tiff` for `.tif`/`.tiff`; `is_photo` covers it.
- `metadata::read_photo_metadata`: `Tiff` → `tiff::read_tiff_metadata` (already reads `DateTimeOriginal`, else `DateTimeDigitized`, else `DateTime`, orientation, camera). Scans without dates fall back to Takeout/file date as now.
- `decode.rs`: TIFF decoded with the `image` crate under `image::Limits { max_alloc: 1 GiB }`, first page only, converted to 8-bit RGB (16-bit, grey, palette, CMYK where the decoder supports it); a crate-wide mutex lets only one TIFF decode at a time (7 GB RAM, two thumbnail workers). Too large or unsupported → `DecodeError`, shown as the existing „cannot load” states.
- Thumbnails: `render_photo` uses `decode_photo` for every kind.

### 3. Full-size JPEG for the viewer (`feat:`)
- `ThumbnailCache` (`crates/core/src/thumbnail.rs`) gains `display_jpeg_for(&Photo) -> Result<Vec<u8>, ThumbnailError>`: full resolution, JPEG quality 90, orientation added with `raw::with_orientation`, stored under the same cache key scheme with a "full size" marker, so `clean` and its size limit cover it.
- `display::display_jpeg` takes the photo and the cache: JPEG → file bytes, RAW → preview, TIFF → cached display JPEG. `photo://` (`src-tauri/src/viewer/photo_protocol.rs`) passes the thumbnail cache from app state.
- The viewer's preload of the next/previous photo warms the cache for TIFFs too (it already requests their `photo://` URLs).

### 4. Fixtures, tests, docs
- `scripts/make-fixtures.py` gains `fixtures/tiff/`: small TIFFs written with Pillow: 8-bit RGB with `DateTimeOriginal` and orientation 6, 16-bit RGB, greyscale, a 2-page TIFF, one without any date, a CMYK one, and a truncated/broken one. `fixtures/README.md` table.
- Tests: metadata per fixture, thumbnails (size, upright), display JPEG cached once (second call reads the cache), multi-page shows page 1, broken file → error, a decode over the memory limit fails cleanly (via a small test limit).
- `docs/domain/photos.md`: TIFF support, first page, viewer JPEG cache; README formats line.
- Status 🧪; debug build; after the user's OK and green CI, release (minor bump) and install.

## Verification
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run notices:rust` unchanged after commit, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`.
- Debug build on a folder in `D:\Lukasz\tmp` with the fixture TIFFs plus a large generated scan (e.g. 8000×6000 16-bit, ~290 MB): thumbnails appear, the viewer opens it, stepping back is instant, memory stays below ~1.5 GB.
- Once unpacked: TIFFs from `F:\Google Photos` opened read-only.

### Changes during implementation

- TIFF photos get a „TIFF” badge like RAW photos (the timeline data needs a format for them anyway).
- `cargo-about` 0.9.3 is now installed locally (same version as CI), so `npm run notices:rust` works on the dev machine; it needs `cargo fetch` first for crates of other targets.
