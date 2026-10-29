# Timeline and viewer

How the grid of photos and the photo viewer behave while they load, for galleries of tens of thousands of photos.

## Thumbnails in the grid

- Every tile is a landscape box; the thumbnail fills it and is cropped to it (portrait photos show a strip from their middle; [slice 12](../slices/12-user-feedback-1.md) plans to show them uncropped).
- Thumbnails load in turns, at most 3 at a time (the backend renders 2 at once; an uncached thumbnail of a 10 MP photo takes about 0.2 s, longer on a USB hard disk):
  1. the tiles on screen, top to bottom and left to right;
  2. then, while nothing visible waits, the tiles the grid renders ahead off screen (4 rows above and below), nearest to the screen first. Loading ahead stops there.
- A tile takes its turn only after it has been rendered for 150 ms, so tiles that only fly past during fast scrolling never load. A tile that scrolls away before its turn never loads either.
- Thumbnails once shown appear at once when their tile comes back. After another folder is opened, thumbnails still queued for the previous one are dropped.
- Thumbnails are cached on disk (up to 500 MB, oldest removed first) and kept when the photo cache is emptied.

## Viewer

- While it waits for the photo and the timeline around it, the viewer shows a spinner; in a folder of 34,600 photos this takes a few seconds ([slice 30](../slices/30-lazy-timeline.md) plans to make it fast).
- Stepping to another photo keeps the previous one for a moment, then shows a spinner if the new one is still loading.

## Staying responsive

- Building the timeline and its series (seconds for tens of thousands of photos) never blocks the app: clicks, scrolling and thumbnails keep working while it reloads.
- During the background EXIF pass of a first scan ([Photos](photos.md#first-scan-of-a-takeout)) the timeline is reloaded at most every 30 s.
