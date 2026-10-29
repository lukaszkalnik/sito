# Slice 12 – User feedback 1

**Status:** ✅ done (tested by the user on 2026-10-29; what it established is in [docs/domain/timeline.md](../domain/timeline.md))

## Context

These findings come from the user's first real test of Sito on their own photos (2026-10-27, with v0.7.x and the slice 9 debug build). Zooming in the viewer, also asked for in that test, is its own slice (13).

## Items

1. **Portrait photos fill the viewer's height,** letterboxed at the sides.
   - The viewer uses `object-fit: contain` on an absolutely positioned `<img>`, so portrait photos should already fill the height.
   - To check first: whether EXIF-rotated photos (stored landscape with an orientation tag) are measured before rotation, or whether a min-size or the series strip shrinks the area.
   - **The grid tiles too** (found 2026-10-29 while testing slice 29): tiles are a fixed landscape box with `object-fit: cover`, so a portrait photo shows only a landscape strip from its middle. In the Takeout's `Zdjęcia z roku 2015`, the `NNN(1).jpg` scans are stored as portrait files (e.g. 1360 × 2008) with the scene lying on its side and no orientation tag (a second, unrotated upload of the landscape `NNN.jpg`). Their middle strip looks like a landscape photo turned by Sito. Showing portrait photos uncropped in their tile (narrower, background at the sides) shows the file's real shape, so such a scan is recognisable as a sideways portrait file; it stays sideways, since nothing in the file says how to turn it.
2. **"Odrzuć" is clearly separated from "Cofnij"** and has a solid red background.
   - Decided 2026-10-29: „Odrzuć” goes to the left end of the viewer bar; „Cofnij” and „Zamknij” stay at the right end.
3. **„Zaakceptuj”** moved to its own [slice 32](32-accept-photo.md) (2026-10-29): it needs its own storage and undo.
4. **The current photo in the series strip** gets a more prominent marker than the thin blue frame, e.g. a thicker accent frame plus a raised or brighter tile, and dimmed neighbours.
5. **The series strip follows:** stepping through a long series scrolls the strip so the current thumbnail stays visible (`scrollIntoView` with `inline: 'nearest'`).
6. **Several cameras at one event** moved to its own [slice 33](33-split-by-camera.md) (2026-10-29): it changes how series are grouped.
7. **„O programie”: dropped** (2026-10-29). „Pomoc → Licencje…” (slice 10) already shows the version number and the license (MIT OR Apache-2.0) with the third-party notices.

## Plan

### 1. `docs: plan slice 12; accept and cameras become slices 32 and 33`
- This plan, the new slice files 32 and 33, and the roadmap rows right after this slice.

### 2. `feat(ui): show portrait thumbnails uncropped in the grid`
- `ui/src/features/timeline/loadedThumbnails.ts`: remember each loaded thumbnail's shape: `rememberLoaded(src, shape)` and `shapeOf(src): ThumbnailShape | undefined` (replaces `wasLoaded`), `type ThumbnailShape = 'landscape' | 'portrait'` (portrait when `naturalHeight > naturalWidth`).
- `TimelineTile.tsx`: `useThumbnailStatus` reads the shape from the `<img>` in `onLoad` and from memory for instant tiles (no flash from cover to contain when a tile comes back); the tile gets `timeline-tile--portrait` and `data-shape`.
- `TimelineGrid.css`: `.timeline-tile--portrait .timeline-tile-image { object-fit: contain; }`, the tile background shows at the sides. Landscape tiles stay `cover`.
- Tests: `loadedThumbnails` shape memory; `TimelineTile` test firing `load` on an image with mocked `naturalWidth/Height` → tile has `data-shape="portrait"`, and a remembered portrait thumbnail is portrait at once.

### 3. `feat(ui): set „Odrzuć” apart from „Cofnij”`
- `PhotoViewer.tsx` `ViewerBar`: the reject button comes first, before the photo info.
- `PhotoViewer.css` `.viewer-reject`: solid red (`#c0392b`, white text, darker on hover, visible focus ring kept).
- Existing tests query by role, so they keep passing; check `ui/e2e/reject.spec.ts` and `viewer.spec.ts`.

### 4. `feat(ui): mark the current photo in the series strip clearly`
- `PhotoViewer.css`: the current strip item gets a 3px accent frame plus a glow (`box-shadow`), neighbours are dimmed (`opacity` about 0.55, back to 1 on hover and focus). Strip padding sized so the frame is never clipped.

### 5. `feat(ui): keep the current photo visible in the series strip`
- `SeriesStrip`: a ref on the current button; an effect on `currentId` calls `scrollIntoView({ inline: 'nearest', block: 'nearest' })` (a DOM sync, so an effect is right).
- Test in `PhotoViewer.test.tsx`: stub `Element.prototype.scrollIntoView`, render, rerender with another current id → called on the new current item with `inline: 'nearest'`.

### 6. Portrait photos in the viewer (item 1)
- Check in the debug build with `fixtures/exif/rotated_orientation-6.jpg` (EXIF-rotated) and a plain portrait JPEG, with and without the series strip.
- If it already fills the height: noted here as confirmed, no code. If not: fix the cause in a separate `fix(ui): …` commit, with a regression test where testable.
- **Result (2026-10-29): confirmed, no code.** Measured in Chromium with the viewer's CSS: `rotated_orientation-6.jpg` (stored 16 × 12) reports as 12 × 16 and fills the full height of the photo area, with and without the strip, also in a tall window. The `photo:` protocol sends the file's own bytes and the webview applies the EXIF orientation. If the user still sees a portrait photo not filling the height, note the file and window size for a sub-slice.

### 7. `docs: slice 12 ready for testing`
- `docs/domain/timeline.md`: grid tiles (portrait shown uncropped) and the viewer (reject at the left, strip marker and follow). Slice and roadmap → 🧪.
- Debug build for the user (`npm run tauri build -- --debug --no-bundle`), only when the user isn't scanning.

### Checks before each commit
`npm run lint`, `npm run typecheck -w ui`, `npm test`; `npm run e2e` before pushing steps 2–5 (they touch the grid and viewer flows). No Rust changes are planned.

## Verification

- In a folder with portrait photos (e.g. the Takeout's `Zdjęcia z roku 2015`, read only), portrait tiles show the whole photo with background at the sides; the sideways `NNN(1).jpg` scans are recognisable as portrait files.
- A portrait photo fills the viewer's height, with and without the series strip.
- „Odrzuć” is red, at the left end of the bar, far from „Cofnij”.
- The current strip thumbnail stands out; stepping through a long series with the arrow keys keeps it visible.
