# Slice 29 – Faster first scan of Takeout folders

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

The first scan of the user's unpacked Google Takeout (34,600 photos on a 5 TB WD My Passport USB hard disk) reads dates at about 1,000 photos per 40 s, so a full first scan takes 35–40 minutes. Later scans are fast because results are cached ([slice 9](09-persistence.md)).

Measured on 2026-10-29 (read-only timing tool on `F:\Google Photos`, fresh photos per run so the file cache does not help):

| What | Time per photo |
|---|---|
| Opening a photo file (no reading) | 30–50 ms |
| Opening and reading the whole photo (1.2 MB) | about the same |
| `nom-exif` reading the EXIF, as Sito does | 30–56 ms |
| Opening and reading its Takeout date file | 10–12 ms |
| The same on the internal NVMe SSD | under 1 ms |
| 1, 2, 4 or all-core parallel reads | no real difference (62–86 ms in total) |
| Finding the files (folder walk, 37k entries) | 34 s |

So the cost is the drive's random access per opened file (a mechanical seek over USB), not the amount of data or Sito's code. The antivirus is off for that folder. The small JSON date files are cheaper to open, probably because NTFS keeps files that small inside its file table.

## Concept

- **Takeout fast path:** for photos with a Takeout date file, the first pass takes the date from the date file only, without opening the photo. The timeline is shown after roughly a fifth of today's time.
- **EXIF later, in the background:** a second pass reads each photo's EXIF (orientation, camera, the time zone comparison of [slice 25](25-takeout-dates.md)) in walk order and updates dates and series through the existing refresh events (`scan-refreshed`, `timeline-changed`).
- **Until EXIF is read,** a photo's time is the Takeout date in the computer's time zone; photos taken abroad may sit a few hours off and then move. Thumbnails need the photo anyway and can read orientation then.
- **Only on slow first scans:** with a cache or on a fast drive, nothing changes.

## Open questions

- Is the time zone shift until the second pass acceptable, or should series wait for the EXIF pass?
- Should the first pass also skip date files and use only the walk (file dates), for galleries without Takeout?
- How is the background pass shown (status line, progress)?

## Verification (outline)

- First scan of `F:\Google Photos`: the timeline appears in a fraction of today's time; after the background pass, dates and series match today's result.
- The timing tool (2026-10-29 measurements) run again for comparison.
