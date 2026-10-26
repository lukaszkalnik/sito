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
| 3 | Timeline | ⬜ |
| 4 | Thumbnails | ⬜ |
| 5 | Time series | ⬜ |
| 6 | Time-gap slider | ⬜ |
| 7 | Compare and decide | ⬜ |
| 8 | Quarantine with undo | ⬜ |
| 9 | Persistence | ⬜ |
| 10 | Sharpness suggestion | ⬜ |
| 11 | Exposure and weights | ⬜ |
| 12 | Split by content | ⬜ |
| 13 | Faces and eyes | ⬜ |
| 14 | macOS polish | ⬜ |

### ✅ 1. Count photos
Pick a folder → the app shows how many photos it found (recursive scan, JPEG only, hidden and `.nomedia` folders skipped, progress while scanning). Plan: [slices/01-count-photos.md](slices/01-count-photos.md)

*Try:* pick your gallery folder, compare the count with Explorer.

### ✅ 2. Installable app
Sito runs like a regular desktop app: a Windows installer (per-user, no admin rights) built by CI on a version tag, with a Start menu entry and desktop shortcut, app icon and name. From now on every slice is testable in the installed app. Plan: [slices/02-installable-app.md](slices/02-installable-app.md)

*Try:* download the installer from the GitHub release, install, start Sito from the Start menu with one click.

### ⬜ 3. Timeline
All photos sorted by capture time (EXIF date and orientation, mtime fallback), shown as a virtualized list or grid with dates and placeholders instead of images. Only visible rows are rendered, so 10,000+ photos scroll smoothly.

*Try:* scroll through a large folder, check order and dates.

### ⬜ 4. Thumbnails
Real thumbnails in the timeline: generated in the background, cached on disk, loaded lazily for the visible area only, correct EXIF rotation.

*Try:* scroll fast through 10,000+ photos; thumbnails fill in without freezing, and a second visit is instant.

### ⬜ 5. Time series
Photos grouped into series by a fixed time gap; single photos hidden; series count shown.

*Try:* check whether the series match what you remember shooting.

### ⬜ 6. Time-gap slider
Change the gap and the series regroup instantly.

*Try:* find the gap value that works for your photos.

### ⬜ 7. Compare and decide
Open a series side by side; arrows navigate, Space = keep, X = reject (decisions in memory only).

*Try:* go through a few series with the keyboard.

### ⬜ 8. Quarantine with undo
Apply decisions: rejected photos move to a quarantine folder, logged in SQLite; undo moves them back.

*Try:* apply on a test folder, undo, verify files are back.

### ⬜ 9. Persistence
SQLite cache of scan results and decisions; rescans are fast and decisions survive a restart.

*Try:* rescan a large folder twice, restart mid-review.

### ⬜ 10. Sharpness suggestion
Sharpness score per photo; the sharpest photo in each series is highlighted as suggested; debug panel shows scores.

*Try:* do the suggestions match your pick?

### ⬜ 11. Exposure and weights
Exposure score added; score weights adjustable in settings.

*Try:* tune weights on series with exposure differences.

### ⬜ 12. Split by content
Embedding model (downloaded on first launch, checksum-verified) splits series with different subjects; similarity slider.

*Try:* series with mixed subjects get split.

### ⬜ 13. Faces and eyes
Face and eye-state score (closed eyes penalized).

*Try:* series of portraits pick shots with open eyes.

### ⬜ 14. macOS polish
Signing, notarization, macOS-specific fixes.
