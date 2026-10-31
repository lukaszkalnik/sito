# Photos

## The galleries

Sito works on large photo galleries taken with Android phones: 10,000+ photos per folder, often in subfolders. Sito reads JPEG (`.jpg`, `.jpeg`) and camera RAW files (`.orf`, `.cr2`, `.nef`, `.arw`, `.rw2`, `.raf`, `.dng`; not `.cr3`); and TIFF (`.tif`, `.tiff`).

## TIFF files

- Scans and reproductions are often TIFF. Metadata comes from the TIFF tags (`DateTimeOriginal`, else `DateTime`; scans often have none and then use a Takeout date or the file date).
- Only the **first page** of a multi-page TIFF is shown; the file is one photo and moves as a whole.
- A browser cannot show TIFF, so the viewer gets a JPEG of at most 4096 px on the long edge, converted once and kept in the thumbnail cache (counted in its size limit). Zooming to 100 % (slice 13) shows that resolution for larger scans. Decoding is limited to 1 GiB of memory and one TIFF at a time; a larger or broken file shows „Nie udało się wczytać zdjęcia.”
- Tiles show a „TIFF” badge.

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

## First scan of a Takeout

Opening each photo file is what makes a first scan slow on a USB hard disk (30–50 ms per file for the seek; a Takeout date file costs about 11 ms), so a first scan of a Takeout reads in two passes:

- **Pass 1** dates every photo that has a Takeout date file from that file alone, without opening the photo, and shows the timeline and series. Photos without a date file are read fully, as always. Without the photo cache everything is read fully.
- **Pass 2** then reads the EXIF of those photos in the background, one at a time in path order, and stores them in the cache in batches of 200. A floating note at the bottom centre of the main window ([Timeline](timeline.md#main-window-bar)) shows „Uzupełnianie dat z EXIF: 1234 z 34 600.”, counting all photos of the folder (so a resumed pass goes on from where it stopped) and updating about once a second; it appears only for folders with Takeout date files, since other photos are read fully in pass 1. The timeline stays usable and is updated at most every 30 s and once at the end.
- **Until pass 2 reaches it**, a photo's time is the Takeout date in the computer's time zone, as for a photo without EXIF. Photos taken in another time zone move by a few hours when their EXIF time replaces it, and their series may change. Thumbnails and the viewer read the orientation from the file, so photos are upright either way.
- Opening another folder stops pass 2; the photos not read yet stay pending in the cache and pass 2 continues when the folder is opened again. Later scans of the same folder read only new or changed files.
- **Scans share the photo cache without waiting for each other:** a scan locks the cache only to look photos up and to store them, never while it walks the folder or reads photos. It stores what it read every 200 photos, so a scan that is stopped (or Sito closed) keeps its work, and the next scan of that folder reads only the rest. Only a complete scan removes photos that are gone from the cache.
- **Opening another folder stops the background work of the previous one:** its background rescan („Sprawdzanie zmian…”, see [Timeline](timeline.md#main-window-bar)) and its pass 2 stop within the next 200 photos (or 500 folder entries), so the new folder's scan starts at once and has the disk to itself.
- Measured on the user's Takeout (34,600 photos, USB hard disk, 2026-10-29): pass 1 took about 5 minutes (before: 35–40 minutes for a full read); pass 2 then read about 70 photos a second in the background.
