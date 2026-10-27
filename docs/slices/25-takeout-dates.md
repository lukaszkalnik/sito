# Slice 25 – Google Takeout dates

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

Photos exported with Google Takeout often have no usable EXIF date (WhatsApp images, screenshots, edited or downloaded copies). After unpacking, their file date is the export date, so Sito would group them all by that meaningless time. Takeout keeps the real capture time in a JSON file next to each photo.

This was split out of slice 20 („Series without EXIF date”) on 2026-10-27, to come right after the README glam shot (slice 11). It doesn't need image similarity.

## What a real Takeout looks like

From the user's own export on `F:\Google Photos`, inspected read-only on 2026-10-27; nothing from it goes into the repo:
- **Five zips:** `takeout-…-1-001.zip` … `-005.zip`, about 50 GB each and 220 GB in total. One zip has about 20,700 entries: 8,100 JPEG, 11,500 JSON, 1,050 Olympus RAW (`.orf`), plus a few TIFF and video files.
- **Paths:** `Takeout/Zdjęcia Google/<album>/<file>`. The folder names are localised (Polish here), and each album has a `metadane.json` (album metadata, localised name).
- **Date files** sit next to each photo: `<name>.<ext>.supplemental-metadata.json`.
  - Duplicate names get numbered date files: `…supplemental-metadata(1).json`, up to `(7)` seen. These presumably belong to `<name>(1).<ext>`; to verify.
  - Older exports and long file names use other forms, e.g. `<name>.<ext>.json` or truncated names.
- **Fields:** `title` (the original file name), `photoTakenTime { timestamp, formatted }` (Unix seconds as a string), `creationTime`, `geoData`, `description`, `url`, `googlePhotosOrigin`.

## Concept

- **Core:** when a photo has no EXIF date, look for its Takeout date file in the same folder and take `photoTakenTime`. That gives a new `TimeSource::Takeout`, which counts like EXIF for series.
  - **Matching:** by the date file's base name first, then the numbered-duplicate rule, then by `title`.
  - **Only the date:** `geoData` and `description` are never read or stored (privacy).
- **Unpacked Takeout:** works through the normal folder scan. The date files are found next to the photos.
- **Zipped Takeout (open):** unpacking 220 GB needs as much free space again.
  - Option A: Sito reads photos directly from the zips (scan, thumbnails, viewer). The photos would be read-only: rejecting can't move a file inside a zip, so it would be a decision list only.
  - Option B: document how to unpack to a drive with room, and support unpacked folders only.
- **Time zone:** `photoTakenTime` is UTC. EXIF capture times are local, so the Takeout time is converted to the local time zone (of the computer, or from the photo's offset when known).
- **Tests:** synthetic fixtures with made-up date files, covering all naming forms. The user's export is used only for a manual check.

## Open questions

- Zips directly (A) or unpack first (B)?
- RAW (`.orf`), TIFF and video: out of scope here? Sito is JPEG-only so far.
- Prefer the Takeout date over a file date only, or also over a suspicious EXIF date (e.g. year 2000 from a camera with a reset clock)?

## Verification (outline)

- Unit tests for every naming form, including duplicates and long names.
- On a copy of one album from the user's Takeout: photos without EXIF show their real dates, and series form across them.
