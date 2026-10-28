# Photos

## The galleries

Sito works on large photo galleries taken with Android phones: 10,000+ photos per folder, often in subfolders. Sito reads JPEG (`.jpg`, `.jpeg`) and camera RAW files (`.orf`, `.cr2`, `.nef`, `.arw`, `.rw2`, `.raf`, `.dng`; not `.cr3`); TIFF is planned (slice 27).

## RAW files

- **Never developed.** Thumbnails and the viewer use the JPEG preview the camera embedded in the RAW file: the largest one Sito finds (in the TIFF structure, in Olympus and Nikon maker notes, Panasonic's `JpgFromRaw`, Fujifilm's RAF header). Older cameras only embed a small thumbnail (e.g. 160×120 on Olympus models from 2003–2006); a file without any preview shows „Nie udało się wczytać zdjęcia.”
- **Metadata** (capture time, orientation, camera) is read from the RAW file's own TIFF structure (Fujifilm RAF and RAW-named JPEGs through their EXIF), so the capture time rules below apply unchanged. The viewer gets the preview with the RAW file's orientation added, so it is shown upright.
- **RAW+JPEG pairs.** A RAW file and a JPEG with the same name (ignoring case) in the same folder are one shot from a camera set to RAW+JPEG: they are one photo, shown and dated by the JPEG, with a „RAW+JPEG” badge (RAW-only photos have a „RAW” badge). Without its own Takeout date file, the pair uses the RAW file's.

## What Sito looks for

**Series of similar shots**: photos taken seconds apart that differ in exposure, slightly in angle, or in facial expression. The user keeps the best one and rejects the rest.

- This is **not** primarily about byte-identical duplicates.
- There are **no** burst or motion photos: every shot is a separate file the user took on purpose.

## Capture time

- The capture time comes from EXIF: `DateTimeOriginal`, else `CreateDate`, else `ModifyDate`. It is the phone's local wall-clock time and is used as such; a time zone offset, when the phone recorded one, is only used to show a Takeout date (below).
- **Implausible EXIF times** count as no capture time: before 1990, exactly January 1 at 00:00:00 (what cameras with a reset clock write), or more than a day in the future.
- **Google Takeout date files.** An unpacked Google Takeout has a JSON date file next to each photo (`IMG.jpg.supplemental-metadata.json`; duplicates `IMG(1).jpg` ↔ `IMG.jpg.supplemental-metadata(1).json`; older exports `IMG.jpg.json`; long names cut short; letter case may differ, e.g. `10(6).jpg` ↔ `10.JPG.supplemental-metadata(6).json`). Photos are paired with them by name, then by the `title` inside. Edited copies (`IMG-edited.jpg`, in Polish exports `IMG-edytowano.jpg`) have no date file of their own and share their original's. Only the capture time (`photoTakenTime`, UTC) and the title are read; places, descriptions and people are never read or stored.
  - No or implausible EXIF time: the Takeout date is used.
  - EXIF and Takeout differ only by a time zone (a multiple of 15 minutes, within ±14 h, give or take 2 s): the EXIF time stays.
  - Any other difference: the date was corrected in Google Photos (e.g. a scan dated 1976, or a camera with a wrong clock), so the Takeout date wins, in any year.
  - A Takeout date is shown in the photo's own EXIF offset if it has one, else in the computer's time zone.
  - The viewer says „data z Google Takeout” or „data poprawiona w Google Photos”; Takeout-dated photos form series like EXIF-dated ones.
  - Only unpacked Takeouts are supported (rejecting needs real files, not entries in a zip).
- Without any capture time, the file's modification time is shown instead and the photo is marked as dated by the file. Such dates usually come from copying or downloading, so they say little about when the photo was taken; these photos never join a series.
- Orientation and camera model are read from EXIF too; photos are always shown upright.
- A new or changed date file makes the next scan read its photo again.
