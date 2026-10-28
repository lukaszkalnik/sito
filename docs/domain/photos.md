# Photos

## The galleries

Sito works on large photo galleries taken with Android phones: 10,000+ photos per folder, often in subfolders. Today only JPEG files (`.jpg`, `.jpeg`) are read; RAW and TIFF are planned (slices 26, 27).

## What Sito looks for

**Series of similar shots**: photos taken seconds apart that differ in exposure, slightly in angle, or in facial expression. The user keeps the best one and rejects the rest.

- This is **not** primarily about byte-identical duplicates.
- There are **no** burst or motion photos: every shot is a separate file the user took on purpose.

## Capture time

- The capture time comes from EXIF: `DateTimeOriginal`, else `CreateDate`, else `ModifyDate`. It is the phone's local wall-clock time and is used as such; a time zone offset, when the phone recorded one, is only used to show a Takeout date (below).
- **Implausible EXIF times** count as no capture time: before 1990, exactly January 1 at 00:00:00 (what cameras with a reset clock write), or more than a day in the future.
- **Google Takeout date files.** An unpacked Google Takeout has a JSON date file next to each photo (`IMG.jpg.supplemental-metadata.json`; duplicates `IMG(1).jpg` ↔ `IMG.jpg.supplemental-metadata(1).json`; older exports `IMG.jpg.json`; long names cut short). Photos are paired with them by name, then by the `title` inside. Only the capture time (`photoTakenTime`, UTC) and the title are read; places, descriptions and people are never read or stored.
  - No or implausible EXIF time: the Takeout date is used.
  - EXIF and Takeout differ only by a time zone (a multiple of 15 minutes, within ±14 h, give or take 2 s): the EXIF time stays.
  - Any other difference: the date was corrected in Google Photos (e.g. a scan dated 1976, or a camera with a wrong clock), so the Takeout date wins, in any year.
  - A Takeout date is shown in the photo's own EXIF offset if it has one, else in the computer's time zone.
  - The viewer says „data z Google Takeout” or „data poprawiona w Google Photos”; Takeout-dated photos form series like EXIF-dated ones.
  - Only unpacked Takeouts are supported (rejecting needs real files, not entries in a zip).
- Without any capture time, the file's modification time is shown instead and the photo is marked as dated by the file. Such dates usually come from copying or downloading, so they say little about when the photo was taken; these photos never join a series.
- Orientation and camera model are read from EXIF too; photos are always shown upright.
- A new or changed date file makes the next scan read its photo again.
