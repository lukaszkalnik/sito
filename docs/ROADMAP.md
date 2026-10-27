# Sito – Roadmap

Work is split into vertical slices. Each slice goes through all layers (core → Tauri → UI) and ends with something the user can try in the running app. Ship one slice at a time; the user tests it before the next one starts.

Plan each slice in plan mode first and save the approved plan as `docs/slices/NN-name.md` (Markdown), then implement. Keep the status emoji in the table, the slice heading and the slice plan in sync.

**Status:** ⬜ not started · 📝 planned · 🚧 in progress · 🧪 ready for testing · ✅ done (tested by the user)

**Scale:** galleries have 10,000+ photos per folder. Every slice must stay responsive at that size (no blocking the UI, no rendering or loading everything at once).

## Slices

| # | Slice | Status |
|---|---|---|
| 1 | [Count photos](slices/01-count-photos.md) | ✅ |
| 2 | [Installable app](slices/02-installable-app.md) | ✅ |
| 3 | [Timeline](slices/03-timeline.md) | ✅ |
| 4 | [Thumbnails](slices/04-thumbnails.md) | ✅ |
| 5 | [Time series](slices/05-time-series.md) | ✅ |
| 6 | [Time-gap slider](slices/06-time-gap-slider.md) | ✅ |
| 7 | [Photo viewer](slices/07-photo-viewer.md) | ✅ |
| 8 | [Reject into "Odrzucone" with undo](slices/08-reject-with-undo.md) | ✅ |
| 9 | [Persistence](slices/09-persistence.md) | 🚧 |
| 10 | Edit series | ⬜ |
| 11 | Settings: trash, cache location and size | ⬜ |
| 12 | Show and restore rejected photos | ⬜ |
| 13 | Sharpness suggestion | ⬜ |
| 14 | Exposure and weights | ⬜ |
| 15 | Split by content | ⬜ |
| 16 | Series without EXIF date | ⬜ |
| 17 | Faces and eyes | ⬜ |
| 18 | English and German UI | ⬜ |
| 19 | macOS polish | ⬜ |
| 20 | Google Photos (to be decided) | ⬜ |

### ✅ 1. Count photos
Pick a folder → the app shows how many photos it found (recursive scan, JPEG only, hidden and `.nomedia` folders skipped, progress while scanning). Plan: [slices/01-count-photos.md](slices/01-count-photos.md)

*Try:* pick your gallery folder, compare the count with Explorer.

### ✅ 2. Installable app
Sito runs like a regular desktop app: a Windows installer (per-user, no admin rights) built by CI on a version tag, with a Start menu entry and desktop shortcut, app icon and name. From now on every slice is testable in the installed app. Plan: [slices/02-installable-app.md](slices/02-installable-app.md)

*Try:* download the installer from the GitHub release, install, start Sito from the Start menu with one click.

### ✅ 3. Timeline
All photos sorted by capture time (EXIF date and orientation, mtime fallback), shown as a virtualized list or grid with dates and placeholders instead of images. Only visible rows are rendered, so 10,000+ photos scroll smoothly. Newest first, grid with day headers. Plan: [slices/03-timeline.md](slices/03-timeline.md)

*Try:* scroll through a large folder, check order and dates.

### ✅ 4. Thumbnails
Real thumbnails in the timeline: generated in the background, cached on disk, loaded lazily for the visible area only, correct EXIF rotation. Generated on demand for visible tiles, cached in the app cache folder. Plan: [slices/04-thumbnails.md](slices/04-thumbnails.md)

*Try:* scroll fast through 10,000+ photos; thumbnails fill in without freezing, and a second visit is instant.

### ✅ 5. Time series
Photos grouped into series by a fixed time gap; single photos hidden; series count shown. Toggle between "Serie" (series only) and "Wszystkie zdjęcia" (full timeline with series marked). Only photos with an EXIF date form series. Plan: [slices/05-time-series.md](slices/05-time-series.md)

*Try:* check whether the series match what you remember shooting.

### ✅ 6. Time-gap slider
Change the gap (1–60 s, with − and + buttons, remembered per folder) and the series regroup instantly, keeping the visible photos in place. Plan: [slices/06-time-gap-slider.md](slices/06-time-gap-slider.md)

*Try:* find the gap value that works for your photos.

### ✅ 7. Photo viewer
Click a thumbnail to open the photo full size in a separate, resizable window (reused for the next photo). If the photo is part of a series, the series thumbnails are shown underneath and open in the same window. Previous/next (buttons and ← →) follow the current view and cross series boundaries. Plan: [slices/07-photo-viewer.md](slices/07-photo-viewer.md)

*Try:* open a photo from a series, click through the strip, step past the end of the series.

### ✅ 8. Reject into "Odrzucone" with undo
A reject button (and the Delete key) in the viewer moves the photo at once into an `Odrzucone` subfolder of its own folder and shows the next one of its series. Photos in `Odrzucone` folders are hidden but still shape the series, so a series sorted down to one photo stays a series; short series are packed side by side. Every move is logged in SQLite; "Cofnij" (Ctrl+Z) moves the last rejected photos back. Plan: [slices/08-reject-with-undo.md](slices/08-reject-with-undo.md)

*Try:* reject a few photos from a test folder, check the `Odrzucone` subfolders, rescan (they stay hidden), undo, verify the files are back.

### 🚧 9. Persistence
SQLite cache of scan results: on start Sito reopens the last folder at once from the cache and checks it for changes in the background; picking a folder seen before is near-instant. Also cleans up the thumbnail cache: removes thumbnails no known photo points to any more (edited, moved or deleted photos) and keeps the cache under 500 MB. Plan: [slices/09-persistence.md](slices/09-persistence.md)

*Try:* restart Sito (the last folder opens at once), rescan a large folder twice, move a folder, rescan, and check that the cache folder stays under 500 MB.

### ⬜ 10. Edit series
Fix false positives and misses by hand: remove a photo from a series, add a photo to a neighbouring series, ungroup a series completely, or "cut here" to split a series that chains several moments (e.g. a long event shot in quick succession) into two at a chosen photo. Edits are stored in SQLite (slice 9), survive rescans and win over automatic grouping.

*Try:* cut a chained series into its real moments, ungroup a wrong series and remove a stray photo, rescan, and check that the edits are kept.

### ⬜ 11. Settings: trash, cache location and size
A settings screen with: moving rejected photos to the system trash instead of the `Odrzucone` folder (logged and undoable via the Recycle Bin; off by default); where the cache lives (thumbnails and `photos.db`; the existing cache is moved over); and the thumbnail cache size limit (default 500 MB).

*Try:* switch the trash option on, reject a few photos, find them in the Recycle Bin, undo; move the cache to another drive and check Sito still starts instantly.

### ⬜ 12. Show and restore rejected photos
Option to show photos from `Odrzucone` folders in the timeline and the viewer, visually weaker (faded, marked "odrzucone") so they do not get in the way. A rejected photo can be restored: moved back from `Odrzucone` to its folder (logged and undoable).

*Try:* show rejected photos, check they are faded but still browsable, restore one and see it back in its folder.

### ⬜ 13. Sharpness suggestion
Sharpness score per photo; the sharpest photo in each series is highlighted as suggested; debug panel shows scores.

*Try:* do the suggestions match your pick?

### ⬜ 14. Exposure and weights
Exposure score added; score weights adjustable in settings.

*Try:* tune weights on series with exposure differences.

### ⬜ 15. Split by content
Embedding model (downloaded on first launch, checksum-verified) splits series with different subjects; similarity slider.

*Try:* series with mixed subjects get split.

### ⬜ 16. Series without EXIF date
Photos dated only by the file (WhatsApp, downloads, copies) often share a copy or download time, so time alone is unreliable for them. Group them by time plus image similarity (embeddings from slice 15). Also reads the date files of a Google Takeout export (`IMG_1234.jpg.json` / `.supplemental-metadata.json`, field `photoTakenTime`), so exported photos without EXIF get their real capture time instead of the export date.

*Try:* a folder of WhatsApp images forms only the series that really belong together. An unpacked Takeout folder shows photos at their real capture dates.

### ⬜ 17. Faces and eyes
Face and eye-state score (closed eyes penalized).

*Try:* series of portraits pick shots with open eyes.

### ⬜ 18. English and German UI
UI translations in English and German next to Polish (`en.json`, `de.json`), with dates and numbers formatted for the chosen language (`en-GB`/`de-DE`) and correct plural rules. The language follows the system language by default and can be changed in settings; a test checks that every key exists in all three files. The rejected folder is named in the chosen language too (`Odrzucone` / `Rejected` / `Abgelehnt`); scans recognise all three names, so photos rejected under one language stay rejected after switching. After a language switch, rejected folders that Sito created itself (recorded in the action log when created) are renamed to the new name, and the log's paths are updated so undo keeps working; folders created by the user keep their name, and a rename is skipped if a folder with the new name already exists.

*Try:* switch the system language (or the setting) to English and German and check all screens, plurals and dates.

### ⬜ 19. macOS polish
Signing, notarization, restoring from the macOS trash, macOS-specific fixes.

### ⬜ 20. Google Photos (to be decided)
Work on photos stored in Google Photos without downloading the whole library. Since March 2025 the official API only allows a hand-picked batch (Picker API) and cannot trash anything. **Preferred direction: a standalone Android app** (Tauri mobile, reusing the core and much of the UI) working on the phone's own photos, with the system trash and undo. Also wanted: the desktop app working directly on a phone attached over USB. Android phones connect over MTP, which on Windows has no drive letter, so this needs the Windows Portable Devices API for reading, thumbnails and moving files into `Odrzucone` on the phone (macOS has no built-in MTP support). Still to verify: whether Google Photos follows trashing done on the phone. Options and open questions: [ideas/google-photos.md](ideas/google-photos.md). Scope still to be decided.
