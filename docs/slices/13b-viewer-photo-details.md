# Slice 13b – Resolution and file size in the viewer

**Status:** ✅ done (tested by the user on 2026-10-30; what it established is in [docs/domain/timeline.md](../domain/timeline.md))

## Context

Asked for by the user on 2026-10-30 while testing [slice 13](13-viewer-zoom.md): the viewer's bar should also show the photo's resolution and file size, next to its name and date.

## Decisions

- **Resolution as the photo is shown, upright:** width × height after the EXIF orientation (a portrait photo reads `3648 × 5472`), e.g. „5472 × 3648”.
- **Where it comes from:** JPEG and TIFF from the image header (only the first bytes of the file are read, nothing is decoded). **RAW photos show no resolution, only the file size:** tried on the 15 real samples in `samples/raw/`, only 3 record an EXIF image size (`ExifImageWidth/Height`), and the Fujifilm ones give the preview's 1280 × 960; a wrong resolution is worse than none. The true RAW size needs each format's own header, a later task.
- **File size** of the photo file itself (for a RAW+JPEG pair the JPEG shown), from the scan (no extra file access), in MB with one decimal (binary megabytes, as Windows Explorer counts them) and a comma, e.g. „7,5 MB”; below 1 MB in kB („812 kB”). Numbers formatted with `pl-PL`.
- Shown after the date in the bar, before the date-source and format notes: „sobota, 1 sierpnia 2026, 19:51:25 · 5472 × 3648 · 7,5 MB”.
- Read for the shown photo only (and the prefetched neighbours, which use the same call), outside the scan lock.

## Plan

### 1. `feat(core): read a photo's resolution`
- New `crates/core/src/dimensions.rs`: `Dimensions { width, height }`, `Dimensions::upright(orientation)` (swaps for orientations 5–8), `read_dimensions(path) -> Option<Dimensions>` (JPEG/TIFF: `image::ImageReader::into_dimensions`; RAW: `None`). Tests on fixtures and for `upright`.

### 2. `feat: resolution and file size in the viewer`
- `src-tauri/src/viewer/page.rs`: `ViewerPhotoDto` gets `details: { sizeBytes, resolution: { width, height } | null }`, read after the scan lock is released. Test.
- `ui/src/features/viewer/api.ts`: the type; `ui/src/features/timeline/format.ts`: `formatFileSize`, `formatResolution` (tests); `PhotoViewer.tsx` shows them in the bar.

### 3. `docs: slice 13b ready for testing`
- `docs/domain/timeline.md` (viewer), slice and roadmap → 🧪.

Checks: Rust fmt/clippy/tests, oxlint, Prettier, tsc, Vitest, Playwright; then a look in the debug build.

## Implementation notes (2026-10-30)

Implemented as planned, except:
- The formatting lives in the viewer feature (`ui/src/features/viewer/photoDetails.ts`, `fileSize`, `formatPixels`) instead of `timeline/format.ts`: only the viewer uses it. Units come from `pl.json` (`viewer.sizeMb`, `viewer.sizeKb`, `viewer.resolution`).
- RAW photos show no resolution (see Decisions); `read_dimensions` returns `None` for them.

Checks green: Rust fmt/clippy/tests, oxlint, Prettier, tsc, Vitest 233, Playwright 47. Not checked in the debug build by computer use: an invisible Huawei window (`hwmdcui.exe`) kept taking the focus.

Tested by the user in the debug build on 2026-10-30: works; released in v0.13.0.
