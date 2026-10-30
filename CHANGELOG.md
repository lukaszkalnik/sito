# Changelog

What changed in each release of Sito, from the user's point of view. The section of a version is published as its GitHub release notes.

## v0.13.0 – 2026-10-30

### New

- **Zoom in the viewer.** Zoom with the mouse wheel or a touchpad pinch around the pointer, `+`/`-`, `0` to fit and `1` for 100 % (one photo pixel per screen pixel, also with Windows display scaling), or double-click to switch between fit and 100 %. Drag or Shift+arrows to move around. The zoom stays while you step through the shots of one series, so you can compare sharpness at the same spot.
- **Reject into the Recycle Bin.** A new choice in the main window, „Odrzucone zdjęcia do: folderu „Odrzucone” / kosza”, sends rejected photos to the system Recycle Bin instead of the „Odrzucone” folder. Switching to the Recycle Bin asks first and is shown in red while active. „Cofnij” brings photos back from either place, also after a restart. Drives without a Recycle Bin (USB sticks, SD cards, network drives) are refused, so nothing is ever deleted for good.
- **„Odrzuć” shows where the photo goes**, with a folder or Recycle Bin icon.
- **Resolution and file size in the viewer**, next to the photo's date (RAW photos: file size only).

### Improved

- **First scan of a large Google Takeout folder is much faster**, especially on a USB hard disk: photos are dated from their Takeout date files right away and their EXIF is read in the background, with progress in the status line.
- **Opening a photo in the viewer is near-instant** in folders with tens of thousands of photos; the viewer now loads only the photo it shows instead of the whole timeline.
- **The app stays responsive while a large photo collection loads or reloads:** clicking, scrolling and thumbnails keep working.
- **Thumbnails appear faster:** the visible ones load first, top to bottom, and thumbnails of a folder you have left are no longer loaded.
- **Portrait photos are shown whole** in the grid instead of cropped to a landscape strip.
- **Clearer viewer controls:** the current photo is clearly marked in the series strip and stays in view while you step through a long series; „Odrzuć” is solid red and set apart from „Cofnij”.

### Fixed

- Double-clicking a photo could open two viewer windows, one stuck on a loading spinner that could not be closed.
- A quick double-click on „Odrzuć” could reject two photos, and holding Delete or Ctrl+Z rejected or restored one photo after another. Each click and each key press now acts once.
- The background EXIF progress now counts on from where it stopped and updates every second.
