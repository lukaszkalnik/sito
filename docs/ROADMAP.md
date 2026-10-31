# Sito – Roadmap

Work is split into vertical slices. Each slice goes through all layers (core → Tauri → UI) and ends with something the user can try in the running app. Ship one slice at a time; the user tests it before the next one starts.

Every slice has its own file, `docs/slices/NN-name.md`, from the moment it is added, and ideas go into `docs/ideas/`. The roadmap only links to those files with each slice's status; it never describes a slice, not even in one line. Plan each slice in plan mode before implementing it and save the approved plan into the same file, then implement. Keep the status emoji in the table, the slice heading and the slice plan in sync.

**Numbers are IDs, not order.** A slice keeps its number forever, like a ticket; a new slice gets the next free number. The order of the table is the order of work: the first slice not done is next. Refer to other slices by number and link.

**Status:** ⬜ not started · 📝 concept drafted or planned · 🚧 in progress · 🧪 ready for testing · ✅ done (tested by the user)

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
| 9 | [Persistence](slices/09-persistence.md) | ✅ |
| 10 | [Third-party license notices](slices/10-third-party-notices.md) | ✅ |
| 11 | [Showcase screenshots for the README](slices/11-readme-showcase.md) | ✅ |
| 25 | [Google Takeout dates](slices/25-takeout-dates.md) | ✅ |
| 26 | [RAW photos](slices/26-raw-photos.md) | ✅ |
| 27 | [TIFF photos](slices/27-tiff-photos.md) | ✅ |
| 27a | [Fixes from testing slice 27](slices/27a-tiff-test-fixes.md) | ✅ |
| 29 | [Faster first scan of Takeout folders](slices/29-fast-takeout-scan.md) | ✅ |
| 29a | [Fixes from testing slice 29](slices/29a-fast-scan-test-fixes.md) | ✅ |
| 30 | [Load the viewer lazily](slices/30-lazy-timeline.md) | ✅ |
| 12 | [User feedback 1](slices/12-user-feedback-1.md) | ✅ |
| 12a | [Fixes from testing slice 12](slices/12a-viewer-test-fixes.md) | ✅ |
| 13 | [Zoom in the viewer](slices/13-viewer-zoom.md) | ✅ |
| 13a | [Fixes from testing slice 13](slices/13a-viewer-zoom-test-fixes.md) | ✅ |
| 13b | [Resolution and file size in the viewer](slices/13b-viewer-photo-details.md) | ✅ |
| 34 | [Reject into the system trash](slices/34-reject-to-trash.md) | ✅ |
| 34a | [Fixes from testing slice 34](slices/34a-reject-to-trash-test-fixes.md) | ✅ |
| 32 | [Accept a photo](slices/32-accept-photo.md) | ✅ |
| 32a | [Fixes from testing slice 32](slices/32a-accept-test-fixes.md) | ✅ |
| 33 | [Several cameras at one event](slices/33-split-by-camera.md) | ✅ |
| 37 | [Steady top bar during background work](slices/37-steady-top-bar.md) | ✅ |
| 40 | [Keep the grid position per folder](slices/40-keep-grid-position.md) | 📝 |
| 36 | [Main window follows the viewer](slices/36-grid-follows-viewer.md) | 🧪 |
| 39 | [Opening a folder while another is still checked](slices/39-switch-folder-during-refresh.md) | 🧪 |
| 38 | [Files changed outside Sito](slices/38-files-changed-outside.md) | 📝 |
| 28 | [End-to-end tests of the real app](slices/28-real-app-e2e.md) | 📝 |
| 14 | [Edit series](slices/14-edit-series.md) | ⬜ |
| 15 | [Settings: cache location and size](slices/15-settings.md) | ⬜ |
| 16 | [Show and restore rejected photos](slices/16-show-rejected.md) | ⬜ |
| 17 | [Sharpness suggestion](slices/17-sharpness.md) | ⬜ |
| 18 | [Exposure and weights](slices/18-exposure-weights.md) | ⬜ |
| 19 | [Split by content](slices/19-split-by-content.md) | ⬜ |
| 20 | [Series without EXIF date](slices/20-series-without-exif-date.md) | ⬜ |
| 21 | [Faces and eyes](slices/21-faces-and-eyes.md) | ⬜ |
| 22 | [English and German UI](slices/22-english-german-ui.md) | ⬜ |
| 23 | [macOS polish](slices/23-macos-polish.md) | ⬜ |
| 24 | [Google Photos (to be decided)](ideas/google-photos.md) | ⬜ |
| 31 | [Load the grid lazily](slices/31-lazy-grid.md) | 📝 |
| 35 | [Install anywhere](slices/35-install-anywhere.md) | 📝 |
