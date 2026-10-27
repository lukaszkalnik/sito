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
- **Unpacked Takeout only** (decided 2026-10-27): it works through the normal folder scan, and the date files are found next to the photos. The README explains how to unpack a Takeout to a drive with enough room. Reading the zips directly isn't supported, because rejecting couldn't move files inside a zip.
- **A wrong EXIF date is overridden** (decided 2026-10-27). Google takes `photoTakenTime` from EXIF on upload and updates it when someone corrects the date in Google Photos, so comparing the two tells which one to use:
  - **Same, or off by whole hours up to ±14 h:** that's only the time zone (EXIF is local, Takeout is UTC). Keep the EXIF time.
  - **Off by anything else:** the date was most likely corrected in Google Photos (e.g. a camera with a reset clock). Use the Takeout time.
  - **Implausible EXIF date** (before 1990, a camera default such as 2000-01-01 00:00 or 1970-01-01, or in the future): use the Takeout time if there is one; otherwise the photo counts as without a date.
  - The photo's info and the debug panel show which date was used and why.
- **Time zone:** `photoTakenTime` is UTC. EXIF capture times are local, so the Takeout time is converted to the local time zone (of the computer, or from the photo's offset when known).
- **Tests:** synthetic fixtures with made-up date files, covering all naming forms. The user's export is used only for a manual check.

## Related slices

- RAW photos (e.g. Olympus `.orf`) are [slice 26](26-raw-photos.md), and TIFF is [slice 27](27-tiff-photos.md). Both come right after this one, so the user's Takeout is fully supported.
- Videos are out of scope.

## Open questions

- Is ±14 h the right limit for "only the time zone"? Daylight-saving shifts and half-hour zones need checking.
- Where to show "date from Takeout" or "date corrected": the viewer's info line, the tile badge (like „data pliku”), or only the debug panel?

## Verification (outline)

- Unit tests for every naming form, including duplicates and long names.
- On a copy of one album from the user's Takeout: photos without EXIF show their real dates, and series form across them.
