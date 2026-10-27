# Slice 26 – RAW photos

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

The user's Google Takeout (see [slice 25](25-takeout-dates.md)) contains about 1,050 Olympus RAW files (`.orf`) in one zip alone, and Sito only reads JPEG so far. RAW files need to work fully: in the timeline, in series, in the viewer, and for rejecting.

## Concept

- **Formats:** Olympus `.orf` first (the user's camera). Other common RAW formats come with the same decoder, if cheap: `.cr2`/`.cr3` (Canon), `.nef` (Nikon), `.arw` (Sony), `.rw2` (Panasonic), `.raf` (Fuji), `.dng`.
- **Capture time and orientation:** read from the RAW's own EXIF/TIFF header (`nom-exif` or the RAW library).
- **Thumbnails and viewer:** most RAW files contain an embedded full-size or large JPEG preview. Using it is fast and looks like the camera's JPEG. Full RAW development (e.g. `rawler`/`rawloader` plus demosaicing) only if a file has no usable preview.
- **The viewer** can't show RAW directly, so the backend serves the embedded preview (or a rendered JPEG) through the `photo://` protocol.
- **RAW + JPEG pairs:** a camera set to RAW+JPEG writes both files (`P1010001.ORF` and `P1010001.JPG`). They are one shot and must not look like a 2-photo series. Proposal: show them as one photo with a „RAW+JPEG” badge, and reject or restore both files together.
- **Sharpness and exposure** (slices 17–18) work on the preview.

## Open questions

- RAW+JPEG pairs: one photo with two files (proposed), or two photos?
- Which RAW crate: license, maintenance, Windows and macOS support, binary size.
- Is the embedded preview good enough for judging sharpness, or is real RAW decoding needed?

## Verification (outline)

- A folder of `.orf` files from the user's Takeout shows correct dates, upright thumbnails, and series.
- The viewer shows them full size.
- A RAW+JPEG pair is one item, and rejecting it moves both files.
