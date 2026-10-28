# Slice 26 – RAW photos

**Status:** ✅ done (tested by the user on 2026-10-28; released as v0.11.0; what it established is in [docs/domain/photos.md](../domain/photos.md) and [docs/domain/rejecting.md](../domain/rejecting.md))

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

# Plan (approved 2026-10-28)

## Decisions and context

The user's Google Takeout has about 1,050 Olympus RAW files (`.orf`) in one zip alone, and Sito only reads JPEG (`is_photo` in `crates/core/src/scan.rs`). RAW files should work fully: timeline, series, thumbnails, viewer, rejecting. Concept: `docs/slices/26-raw-photos.md`.

User decisions (2026-10-28):
- **RAW+JPEG pair = one photo** with both files: one tile with a „RAW+JPEG” badge; reject and undo move both files.
- **Formats:** `.orf` first, plus `.cr2`, `.nef`, `.arw`, `.rw2`, `.raf`, `.dng` where the embedded preview reads the same way. `.cr3` (ISO-BMFF, different container) is left out unless trivial.
- **Test data:** CC0 samples (e.g. raw.pixls.us) may be downloaded, kept out of the repo; small synthetic fixtures are committed.

Findings that shape the design:
- `nom-exif` 3.8 rejects ORF (`IIRO`/`IIRS` magic instead of `II*\0`) and RW2 (`IIU\0`), and extracts no previews.
- RAW libraries are LGPL-2.1 (`rawler`, `rawloader`) or AGPL (`zenraw`): unsuitable for static linking into an MIT/Apache app. So **a small TIFF reader of our own** (no new dependency) reads the few metadata tags and finds the embedded JPEG preview. Full RAW development (demosaicing) is out of scope: a file without a usable preview shows the „cannot load” state.
- The thumbnail pipeline decodes from a path (`decode_scaled`/`decode_full` in `crates/core/src/thumbnail.rs`); `photo://` serves the raw file bytes (`src-tauri/src/viewer/photo_protocol.rs`), and the webview applies EXIF orientation itself.

## Shared with slice 27 (TIFF)

Most RAW formats are TIFF containers, and both slices turn a non-JPEG file into something the webview and the thumbnailer can use. These pieces are built here, generic, so slice 27 only adds a TIFF arm:

| Piece | Slice 26 | Slice 27 adds |
|---|---|---|
| `PhotoKind { Jpeg, Raw(RawFormat), Tiff }` and `photo_kind(path)` in `scan.rs` | `Jpeg`, `Raw` recognised; `Tiff` exists but is not yet a photo | recognise `.tif`/`.tiff` |
| `crates/core/src/tiff/` IFD walker (endianness, IFD chain, SubIFDs, Exif IFD, bounded seek reads, cycle guard) | used by RAW metadata and previews | TIFF metadata (`DateTimeOriginal`, else `DateTime`, which scans often only have) |
| `metadata::read_photo_metadata` dispatching by `PhotoKind`, TIFF-family files read by the IFD walker | JPEG → nom-exif, RAW → walker | TIFF → walker |
| `ImageSource { Path, Bytes }` for thumbnails | RAW preview bytes | TIFF path, decoded with the `image` crate's `tiff` feature and a memory cap |
| `display::display_jpeg(photo) -> Result<DisplayJpeg, DisplayError>` in core, used by `photo://` | JPEG → file bytes; RAW → preview + orientation APP1 | TIFF → rendered full-size JPEG, kept in a display cache next to the thumbnail cache |
| „cannot load” path in the viewer for files without a usable image | RAW without preview | TIFF that fails to decode / exceeds the cap |

Slice 27's concept file gets a short note pointing at these pieces.

## Implementation

### 0. Docs
- **CLAUDE.md rule** (user, 2026-10-28), in "Where knowledge lives" next to the slice rules: *When planning or implementing a slice, read the concepts of related later slices (same files, same data, same kind of feature) and build the shared pieces generically now, so the later slice only adds its own part; note in the later slice's file what it can build on.* Commit `docs: consider related future slices when planning one`.
- Save this plan into `docs/slices/26-raw-photos.md` (concept kept above), status 🚧 there and in `docs/ROADMAP.md`. Commit `docs: add slice 26 plan`.

### 1. Refactor: decode thumbnails from bytes too (`refactor:`)
`render_thumbnail` and `decode_scaled`/`decode_full` (`crates/core/src/thumbnail.rs`) take an `ImageSource { Path(&Path) | Bytes(&[u8]) }` (`jpeg_decoder` over a `Cursor` for bytes). No behaviour change.

### 2. Core: RAW reading (`crates/core/src/raw.rs` + `raw/`; `feat:`)
- `raw/format.rs` (pure): `RawFormat { Orf, Cr2, Nef, Arw, Rw2, Raf, Dng }` from the extension; header check by magic (`II*\0`/`MM\0*`, ORF `IIRO`/`IIRS`/`MMOR`, RW2 `IIU\0`, RAF `FUJIFILMCCD-RAW`).
- `crates/core/src/tiff/` (shared, data, seek-based, reads only the bytes it needs, bounded): IFD walker with endianness and any TIFF-like magic; follows IFD chain, `SubIFDs` (0x014A), Exif IFD (0x8769). The Olympus MakerNote (`OLYMPUS\0` header, CameraSettings 0x2020) is a RAW-specific step in `raw/`.
- `tiff/metadata.rs` (shared): `DateTimeOriginal`/`OffsetTimeOriginal` (else `DateTime`), `Orientation`, `Make`/`Model` → the existing `PhotoMetadata` (`crates/core/src/metadata.rs`), so the capture-time rules (plausibility, Takeout) apply unchanged. RAF: header points to its embedded JPEG; metadata comes from that JPEG's EXIF via `nom_exif::read_exif_from_bytes`.
- `raw/preview.rs`: candidate JPEGs: `JPEGInterchangeFormat`/`Length` in every IFD, `StripOffsets` of JPEG-compressed IFDs (CR2 IFD0, DNG/NEF previews with NewSubfileType=1), Olympus `PreviewImageStart`/`Length` (relative to the MakerNote), RW2 `JpgFromRaw` (0x002E), RAF header offset. Keep those that start with `FFD8` and whose frame is baseline/progressive (`jpeg_decoder::read_info`, rejects lossless-JPEG raw data); pick the largest. `read_preview(path) -> Result<Vec<u8>, RawError>`.
- `with_orientation(jpeg, orientation) -> Vec<u8>` (pure): splices a minimal EXIF APP1 with the Orientation tag after SOI when the preview has none, so the webview rotates it like a camera JPEG (cheap, no re-encode).
- `read_photo_metadata` dispatches by file kind; `thumbnail` decodes RAW via `read_preview` + `ImageSource::Bytes`.
- Tests: synthetic fixtures (below) for each layout; unit tests for the IFD walker (both endiannesses, cycles and out-of-range offsets refused), candidate choice, APP1 splice.

### 3. RAW on the timeline and in the viewer (`feat:`)
- `scan.rs`: `is_photo` → `photo_kind(path) -> Option<PhotoKind { Jpeg, Raw(RawFormat) }>` (case-insensitive extensions); `is_photo` stays as `photo_kind(..).is_some()`. Takeout pairing works unchanged (names are generic).
- `crates/core/src/display.rs`: `display_jpeg(photo)` dispatches by `PhotoKind` (JPEG → file bytes; RAW → `read_preview` + `with_orientation`). `photo://` (`src-tauri/src/viewer/photo_protocol.rs`) calls it instead of reading the file; no usable image → error status, the viewer shows its existing load error.
- `thumb://` works through `render_thumbnail`.

### 4. RAW+JPEG pairs as one photo (`feat:`)
- Pairing in `find_photos` after the walk, per folder: a RAW and a JPEG with the same stem (ignoring case) become one `FoundFile` whose primary file is the **JPEG** (camera-rendered, fast, has orientation) with `companion: Option<FoundCompanion { path, stamp }>` (the RAW). Pairs with a Takeout `-edytowano` copy are not affected (different stem).
- `Photo.companion: Option<Companion { path, stamp }>`; photo cache: `CACHE_VERSION` 2 → 3 with `companion_path`, `companion_size`, `companion_mtime_ns`; `is_unchanged` compares it.
- DTO: `TimelineEntryDto.files: 'single' | 'rawJpeg'` plus `raw: bool` for RAW-only → tile badges „RAW” and „RAW+JPEG” (`pl.json`), and the same in the viewer info line.
- Domain doc + e2e mock updated.

### 5. Rejecting moves every file of a photo (`refactor:` + `feat:`)
- `reject.rs`: `Rejected` holds the primary move plus `extra: Vec<FileMove>` (photo's own Takeout file, the companion, the companion's own Takeout file), replacing `sidecar`. Names in „Odrzucone”: primary and companion get one common free suffix (`P1 (2).JPG` + `P1 (2).ORF`), Takeout files are named after their photo file. Extra files move first; the primary last; on failure everything moved is moved back in reverse.
- `actions.rs`: migration 3 adds `action_moves(action_id, from_path, to_path)`; `record_reject` writes extras there; `last_undoable` reads them plus the old `sidecar_*` columns of older entries.
- Undo: primary back first (required), then extras best effort, each failure reported (`Restored.errors`).
- Tests: pair reject/undo, name collision gives a common suffix, rollback, old log entries with sidecar columns still undo.

### 6. Fixtures, samples, docs
- `scripts/make-fixtures.py` gains `fixtures/raw/`: tiny synthetic files per layout (ORF with MakerNote preview, CR2 with IFD0 JPEG strip, NEF/DNG with a SubIFD preview, ARW with JPEGInterchangeFormat, RW2 with JpgFromRaw, RAF header) each embedding a 32×24 JPEG and known date/orientation/camera, plus a RAW+JPEG pair and a RAW without preview. `fixtures/README.md` table.
- `scripts/get-raw-samples.py`: downloads a few CC0 samples per format into `samples/raw/` (git-ignored) after checking each file's license on raw.pixls.us; an `#[ignore]` integration test (`cargo test -- --ignored`) reads every sample: metadata present, preview decodes, larger than 1000 px.
- `docs/domain/photos.md` (formats, previews, pairs, no full RAW development), `docs/domain/rejecting.md` (all files of a photo move together), README formats line, and a "Builds on slice 26" note in `docs/slices/27-tiff-photos.md` listing the shared pieces.
- Status 🧪; after CI is green, release (minor bump) and silent install.

## Verification
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo test -- --ignored` with the downloaded samples, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`.
- Debug build: a folder with the CC0 samples and a RAW+JPEG pair copied to `D:\Lukasz\tmp`: dates, upright thumbnails, viewer full size, „RAW+JPEG” as one tile, reject + undo moves both files.
- Once the Takeout is unpacked: an album with `.orf` files from `F:\Google Photos` opened read-only.

### Changes during implementation

- `PhotoKind` has no `Tiff` variant yet: an unused variant would need dead arms everywhere; slice 27 adds it (noted there).
- Checked against 15 CC0 samples from raw.pixls.us: added Nikon maker note previews, the old Olympus maker note thumbnail, and RAW-named files that are really JPEGs (a Samsung `.dng`). 13 of 15 show a preview; a cinema-camera DNG and a CHDK file have none.
- Sample names contain `:` (`(4:3)`), which NTFS takes for an alternate data stream; the download script replaces characters Windows forbids.
