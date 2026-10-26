# Sito – Roadmap

Work is split into vertical slices. Each slice goes through all layers (core → Tauri → UI) and ends with something the user can try in the running app. Ship one slice at a time; the user tests it before the next one starts.

Plan each slice in plan mode first, then implement. Mark a slice as done here when the user has tested it.

## Slices

| # | Slice | Status |
|---|---|---|
| 1 | Count photos | ⬜ |
| 2 | Timeline grid | ⬜ |
| 3 | Time series | ⬜ |
| 4 | Time-gap slider | ⬜ |
| 5 | Compare and decide | ⬜ |
| 6 | Quarantine with undo | ⬜ |
| 7 | Persistence | ⬜ |
| 8 | Sharpness suggestion | ⬜ |
| 9 | Exposure and weights | ⬜ |
| 10 | Split by content | ⬜ |
| 11 | Faces and eyes | ⬜ |
| 12 | macOS polish | ⬜ |

### 1. Count photos
Pick a folder → the app shows how many photos it found (recursive scan, JPEG/HEIC/PNG by extension, progress while scanning).
*Try:* pick your gallery folder, compare the count with Explorer.

### 2. Timeline grid
Thumbnails of all photos sorted by capture time (EXIF date and orientation, mtime fallback; thumbnail cache).
*Try:* scroll through the gallery, check order and rotation.

### 3. Time series
Photos grouped into series by a fixed time gap; single photos hidden; series count shown.
*Try:* check whether the series match what you remember shooting.

### 4. Time-gap slider
Change the gap and the series regroup instantly.
*Try:* find the gap value that works for your photos.

### 5. Compare and decide
Open a series side by side; arrows navigate, Space = keep, X = reject (decisions in memory only).
*Try:* go through a few series with the keyboard.

### 6. Quarantine with undo
Apply decisions: rejected photos move to a quarantine folder, logged in SQLite; undo moves them back.
*Try:* apply on a test folder, undo, verify files are back.

### 7. Persistence
SQLite cache of scan results and decisions; rescans are fast and decisions survive a restart.
*Try:* rescan a large folder twice, restart mid-review.

### 8. Sharpness suggestion
Sharpness score per photo; the sharpest photo in each series is highlighted as suggested; debug panel shows scores.
*Try:* do the suggestions match your pick?

### 9. Exposure and weights
Exposure score added; score weights adjustable in settings.
*Try:* tune weights on series with exposure differences.

### 10. Split by content
Embedding model (downloaded on first launch, checksum-verified) splits series with different subjects; similarity slider.
*Try:* series with mixed subjects get split.

### 11. Faces and eyes
Face and eye-state score (closed eyes penalized).
*Try:* series of portraits pick shots with open eyes.

### 12. macOS polish
Signing, notarization, macOS-specific fixes.

## Releases

An installable Windows build (tag `v*` → draft release with MSI) can be cut after any slice to test outside the dev setup.
