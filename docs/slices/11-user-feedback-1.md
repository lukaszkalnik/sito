# Slice 11 – User feedback 1

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

These findings come from the user's first real test of Sito on their own photos (2026-10-27, with v0.7.x and the slice 9 debug build). Zooming in the viewer, also asked for in that test, is its own slice (12).

## Items

1. **Portrait photos fill the viewer's height,** letterboxed at the sides.
   - The viewer uses `object-fit: contain` on an absolutely positioned `<img>`, so portrait photos should already fill the height.
   - To check first: whether EXIF-rotated photos (stored landscape with an orientation tag) are measured before rotation, or whether a min-size or the series strip shrinks the area.
2. **"Odrzuć" is clearly separated from "Cofnij"** and has a solid red background.
   - Placing it apart, e.g. at the other end of the bar or next to the photo, avoids pressing the wrong one.
3. **"Zaakceptuj"** marks a photo as accepted, e.g. as the first pick in a large series.
   - Button and key (to decide: `A` or `Enter`).
   - A green check mark appears on the photo's **thumbnails only** (timeline and series strip), not on the photo in the viewer.
   - Stored in SQLite next to the action log, so it survives restarts, and undoable like a rejection.
   - Open: may several photos of a series be accepted? Does accepting the last one move on to the next series?
4. **The current photo in the series strip** gets a more prominent marker than the thin blue frame, e.g. a thicker accent frame plus a raised or brighter tile, and dimmed neighbours.
5. **The series strip follows:** stepping through a long series scrolls the strip so the current thumbnail stays visible (`scrollIntoView` with `inline: 'nearest'`).
6. **Several cameras at one event.** When their clocks are slightly off, their series interleave (2 photos outside, 2 inside from another camera, then back outside).
   - Proposal: split series by camera (EXIF make and model), even when their times overlap. Core already reads `camera`.
   - Open: how overlapping series are ordered (e.g. by their newest photo, as the user suggested).
   - Open: how the header shows the overlap (e.g. „nakłada się z inną serią”, or the camera name).
   - Open: photos without camera info.
7. **„O programie”** shows the version number, the license (MIT OR Apache-2.0), and the third-party notices from slice 10.

## Verification (outline)

- A portrait photo fills the viewer's height.
- The best shot of a big series can be accepted; its check mark shows in the timeline and in the strip, and survives a restart.
- Stepping through a long series keeps the current thumbnail visible in the strip.
- The party folder shot with several cameras keeps each camera's series together.
- „O programie” shows the right version.
