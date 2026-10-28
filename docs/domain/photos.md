# Photos

## The galleries

Sito works on large photo galleries taken with Android phones: 10,000+ photos per folder, often in subfolders. Today only JPEG files (`.jpg`, `.jpeg`) are read; RAW and TIFF are planned (slices 26, 27).

## What Sito looks for

**Series of similar shots**: photos taken seconds apart that differ in exposure, slightly in angle, or in facial expression. The user keeps the best one and rejects the rest.

- This is **not** primarily about byte-identical duplicates.
- There are **no** burst or motion photos: every shot is a separate file the user took on purpose.

## Capture time

- The capture time comes from EXIF: `DateTimeOriginal`, else `CreateDate`, else `ModifyDate`. It is the phone's local wall-clock time and is used as such; a time zone offset, when the phone recorded one, is read but not used.
- Without a usable EXIF date, the file's modification time is shown instead and the photo is marked as dated by the file. Such dates usually come from copying or downloading, so they say little about when the photo was taken.
- Orientation and camera model are read from EXIF too; photos are always shown upright.
