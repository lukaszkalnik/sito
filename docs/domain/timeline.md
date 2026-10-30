# Timeline and viewer

How the grid of photos and the photo viewer show photos and behave while they load, for galleries of tens of thousands of photos.

## Main window bar

- The bar above the grid keeps its height: the folder path and the summary (photos, series, photos without a date, unreadable entries) are one line each, cut with an ellipsis in a narrow window, with the full text as a tooltip.
- Notes about background work float in a small pill at the bottom centre of the window, over the grid: „Sprawdzanie zmian…” while a reopened folder is checked for changes, and the progress of the EXIF pass of a first scan ([Photos](photos.md#first-scan-of-a-takeout)); both can show together. Clicks go through the pill to the tiles, and the grid ends with some empty space, so the last row can be scrolled above it. The undo toast shows at the top centre, just below the bar.

## Series in the grid

- Each series has a header with its photo count and time range (e.g. „3 zdjęcia · 17:00:11–17:00:22”); when it overlaps a series of another camera, the header also names its camera ([Grouping](grouping.md#several-cameras-at-one-event)). A series that fits on one row is a tinted block next to other blocks and single photos.

## Thumbnails in the grid

- Every tile is a landscape box. A landscape thumbnail fills it and is cropped to it; a portrait thumbnail (taller than wide, after the EXIF orientation) is shown whole, with the tile background at its sides. So a portrait file whose scene lies on its side (e.g. a sideways scan without an orientation tag) is recognisable as such; Sito does not turn it, since nothing in the file says how.
- A tile knows a thumbnail's shape once it has loaded; a thumbnail shown before keeps its shape when its tile comes back, without changing from cropped to whole.
- Thumbnails load in turns, at most 3 at a time (the backend renders 2 at once; an uncached thumbnail of a 10 MP photo takes about 0.2 s, longer on a USB hard disk):
  1. the tiles on screen, top to bottom and left to right;
  2. then, while nothing visible waits, the tiles the grid renders ahead off screen (4 rows above and below), nearest to the screen first. Loading ahead stops there.
- A tile takes its turn only after it has been rendered for 150 ms, so tiles that only fly past during fast scrolling never load. A tile that scrolls away before its turn never loads either.
- Thumbnails once shown appear at once when their tile comes back. After another folder is opened, thumbnails still queued for the previous one are dropped.
- Thumbnails are cached on disk (up to 500 MB, oldest removed first) and kept when the photo cache is emptied.

## Viewer

- The viewer loads one photo at a time, never the whole timeline: for the photo shown it gets the photo, the photos of its series (the strip), the previous and next photo of the view and the photo to show after rejecting it. The backend works these out from the series of the current gap, which it finds once per change of the timeline.
- Until the photo's details arrive, the viewer shows a spinner.
- The details of the previous and next photo are fetched ahead, together with the photos themselves, so stepping needs no wait. Fetched details are kept until the view (mode, gap) or the timeline changes.
- Stepping to another photo keeps the previous one for a moment, then shows a spinner if the new one is still loading.
- When the timeline changes (a rejection or undo in either window, a refresh, the EXIF pass), the viewer fetches the shown photo's details again.
- The photo fills the space between the bar and the strip (or the bottom edge, when it has no series) as large as it fits, uncropped, so a portrait photo fills the height. The webview applies the EXIF orientation, so a rotated photo is measured upright.
- At the right end of the bar, „Zaakceptuj” (green outline, solid green when the photo is accepted; see [Accepting](accepting.md)) comes before „Odrzuć” (solid red), which is set about one button width apart from „Cofnij” and „Zamknij”, so the two are not hit by mistake.
- After the date, the bar shows the photo's **resolution** as it is shown (after the EXIF orientation, e.g. „5472 × 3648”) and its **file size** („7,5 MB”, binary megabytes with one decimal as Windows Explorer counts them; below 1 MB in whole kB). The resolution comes from the JPEG or TIFF header (nothing is decoded); RAW photos show only the file size, since most RAW files record no reliable size outside their per-camera headers.
- In the series strip, the current photo has a thick accent frame with a glow; the other photos are shown normally. Accepted photos carry a green check, in the strip as on their grid tiles. Stepping through a long series scrolls the strip just enough to keep the current photo visible.
- One click on a tile opens the viewer; there is only ever one viewer window, also when a tile is double-clicked. A viewer window that is still loading (or reloads) gets the last photo asked for from the backend.
- **Double-clicks and held keys act once** where acting twice would harm: a double-click on „Zaakceptuj”, „Odrzuć”, „Cofnij”, „Zamknij” or a tile counts as one click (keyboard activation always counts), and holding A, Delete, Ctrl+Z or Escape does not repeat them. Held arrow keys (stepping, panning) and the zoom keys do repeat.

### Zoom

- The mouse wheel and a touchpad pinch zoom around the pointer. `+`/`-` zoom by ×1.25 around the middle of the view, `0` fits, `1` shows 100 %, and a double-click switches between fit and 100 % at the clicked spot. The webview's own page zoom never kicks in over the photo.
- 100 % means one photo pixel per screen pixel, also with Windows display scaling, so sharpness can be judged on real pixels. A RAW photo shows its embedded preview ([Photos](photos.md)), so there 100 % means the preview's pixels.
- The zoom goes from fit up to 800 %; zooming out past fit snaps back to fit. Fit is the photo as large as it fits, uncropped (a photo smaller than the view is enlarged to fit, as before zoom existed); a photo whose fit is already above 100 % cannot be zoomed further in by `1`.
- While zoomed, dragging pans the photo (grab cursor) and Shift+arrows pan by 120 pixels; plain arrows still step to the previous or next photo. The photo can't be moved past its edges, and a side smaller than the view stays centred.
- The position is kept relative to the photo (the spot in the middle of the view, as a fraction of its width and height), so resizing the window or stepping to a photo of another size keeps the same spot in view.
- Zoom and position carry over to the other photos of the same series, also after a rejection that shows the next shot of the series. Stepping into another series, or onto a photo without a series, resets to fit.
- While zoomed, a badge in the top-right corner shows the level (e.g. „100%”) and a „Dopasuj” button that goes back to fit. Nothing is shown at fit.

## Staying responsive

- Building the timeline and its series (seconds for tens of thousands of photos) never blocks the app: clicks, scrolling and thumbnails keep working while it reloads.
- During the background EXIF pass of a first scan ([Photos](photos.md#first-scan-of-a-takeout)) the timeline is reloaded at most every 30 s.
