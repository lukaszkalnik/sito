# Timeline and viewer

How the grid of photos and the photo viewer show photos and behave while they load, for galleries of tens of thousands of photos.

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
- At the right end of the bar, „Odrzuć” (solid red) is set about one button width apart from „Cofnij” and „Zamknij”, so the two are not hit by mistake.
- In the series strip, the current photo has a thick accent frame with a glow; the other photos are shown normally. Stepping through a long series scrolls the strip just enough to keep the current photo visible.

## Staying responsive

- Building the timeline and its series (seconds for tens of thousands of photos) never blocks the app: clicks, scrolling and thumbnails keep working while it reloads.
- During the background EXIF pass of a first scan ([Photos](photos.md#first-scan-of-a-takeout)) the timeline is reloaded at most every 30 s.
