# Changelog

What changed in each release of Sito, from the user's point of view. The section of a version is published as its GitHub release notes.

## v0.14.0 – 2026-10-31

### New

- **Accept the keepers of a series.** „Zaakceptuj” in the viewer (or the key A) marks the shown photo as accepted; accepted photos carry a green check in the grid and in the viewer's series strip. Accepting never touches the file, the mark survives restarts, and „Cofnij” / Ctrl+Z undo it like a rejection. ([slice 32](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/32-accept-photo.md), [32a](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/32a-accept-test-fixes.md))
- **The main window follows the viewer.** The photo shown in the viewer is framed in the grid, which scrolls to it when it is out of view (after a short pause, so holding an arrow key stays smooth). When the viewer closes, that photo pulses for a few seconds so you find your place. ([slice 36](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/36-grid-follows-viewer.md))

### Improved

- **Several cameras at one event are kept apart:** a series now holds the photos of one camera only, even when two cameras shot the same moment, and is shown in one piece. A series that overlaps another camera's series names its camera in its header (e.g. „· Canon EOS R6”). ([slice 33](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/33-split-by-camera.md))
- **The top bar keeps its height** while Sito works in the background: „Sprawdzanie zmian…” and the EXIF progress float in a small note at the bottom of the window instead of making the bar and the grid jump. ([slice 37](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/37-steady-top-bar.md))

### Fixed

- Choosing another folder could hang at „znaleziono 0 zdjęć” for many minutes while Sito was still checking the previously opened folder for changes (for example after its subfolders were renamed). Opening a folder now stops that check right away, and a long check keeps what it has read if it is stopped or Sito is closed. ([slice 39](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/39-switch-folder-during-refresh.md))

## v0.13.0 – 2026-10-30

### New

- **Zoom in the viewer.** Zoom with the mouse wheel or a touchpad pinch around the pointer, `+`/`-`, `0` to fit and `1` for 100 % (one photo pixel per screen pixel, also with Windows display scaling), or double-click to switch between fit and 100 %. Drag or Shift+arrows to move around. The zoom stays while you step through the shots of one series, so you can compare sharpness at the same spot. ([slice 13](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/13-viewer-zoom.md))
- **Reject into the Recycle Bin.** A new choice in the main window, „Odrzucone zdjęcia do: folderu „Odrzucone” / kosza”, sends rejected photos to the system Recycle Bin instead of the „Odrzucone” folder. Switching to the Recycle Bin asks first and is shown in red while active. „Cofnij” brings photos back from either place, also after a restart. Drives without a Recycle Bin (USB sticks, SD cards, network drives) are refused, so nothing is ever deleted for good. ([slice 34](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/34-reject-to-trash.md))
- **„Odrzuć” shows where the photo goes**, with a folder or Recycle Bin icon. ([slice 34a](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/34a-reject-to-trash-test-fixes.md))
- **Resolution and file size in the viewer**, next to the photo's date (RAW photos: file size only). ([slice 13b](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/13b-viewer-photo-details.md))

### Improved

- **First scan of a large Google Takeout folder is much faster**, especially on a USB hard disk: photos are dated from their Takeout date files right away and their EXIF is read in the background, with progress in the status line. ([slice 29](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/29-fast-takeout-scan.md), [29a](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/29a-fast-scan-test-fixes.md))
- **Opening a photo in the viewer is near-instant** in folders with tens of thousands of photos; the viewer now loads only the photo it shows instead of the whole timeline. ([slice 30](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/30-lazy-timeline.md))
- **The app stays responsive while a large photo collection loads or reloads:** clicking, scrolling and thumbnails keep working. ([slice 29a](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/29a-fast-scan-test-fixes.md))
- **Thumbnails appear faster:** the visible ones load first, top to bottom, and thumbnails of a folder you have left are no longer loaded. ([slice 29a](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/29a-fast-scan-test-fixes.md))
- **Portrait photos are shown whole** in the grid instead of cropped to a landscape strip. ([slice 12](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/12-user-feedback-1.md))
- **Clearer viewer controls:** the current photo is clearly marked in the series strip and stays in view while you step through a long series; „Odrzuć” is solid red and set apart from „Cofnij”. ([slice 12](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/12-user-feedback-1.md), [12a](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/12a-viewer-test-fixes.md))

### Fixed

- Double-clicking a photo could open two viewer windows, one stuck on a loading spinner that could not be closed. ([slice 13a](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/13a-viewer-zoom-test-fixes.md))
- A quick double-click on „Odrzuć” could reject two photos, and holding Delete or Ctrl+Z rejected or restored one photo after another. Each click and each key press now acts once. ([slice 13a](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/13a-viewer-zoom-test-fixes.md))
- The background EXIF progress now counts on from where it stopped and updates every second. ([slice 29a](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/29a-fast-scan-test-fixes.md))
