# Slice 27 – TIFF photos

**Status:** 📝 draft (concept; detailed plan when the slice starts)

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
