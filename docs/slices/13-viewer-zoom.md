# Slice 13 – Zoom in the viewer

**Status:** ✅ done (tested by the user on 2026-10-30; what it established is in [docs/domain/timeline.md](../domain/timeline.md))

## Context

This was asked for in the user's first test (2026-10-27): to choose the best shot, sharpness and faces have to be compared at 100 %, which the fitted view can't show.

## Decisions

Decided by the user on 2026-10-30:
- Zoom and position carry over **within a series**. Stepping into another series, or onto a photo with no series, **resets to fit**.
- **Shift+arrows pan** while zoomed in. Plain arrows still step to the previous or next photo.
- **Double-click** switches between fit and 100 % at the clicked spot.
- Side-by-side comparison is out of scope; it goes into `docs/ideas/` as a later idea.

## Behaviour

- Wheel and touchpad pinch (WebView2 sends a pinch as Ctrl+wheel) zoom around the pointer. `+`/`-` zoom by ×1.25 around the middle of the view, `0` = fit, `1` = 100 %.
- Zoom range: from fit up to 800 % of the photo's own pixels. Zooming out past fit snaps back to fit. Photos smaller than the view are not enlarged when fitted (as today).
- Drag pans the photo (grab cursor). The photo can't be dragged past its edges, and an axis smaller than the view stays centred.
- The position is stored relative to the photo (the centre of the view as a fraction of its width and height), so stepping to the next shot of a series, or resizing the window, keeps the same spot in view.
- The zoom level is stored relative to the photo's own pixels, so 100 % always means real pixels. A RAW photo shows its embedded preview, so there 100 % means the preview's pixels; this gets noted in the domain doc.
- While zoomed, a small overlay in the photo's top-right corner shows the level (`Intl.NumberFormat('pl-PL', { style: 'percent' })`, e.g. „100 %”) and a „Dopasuj” button that goes back to fit. Nothing is shown at fit.
- While the next photo loads, the previous one is shown with the same zoom (today's `WhileLoading` already keeps it on screen).

## Plan

1. **`refactor:` move `FullPhoto`, `WhileLoading` and `LoadingSpinner`** out of `ui/src/features/viewer/PhotoViewer.tsx` (281 lines) into `FullPhoto.tsx`, so the zoom has room. No change in behaviour.
2. **`feat:` shortcuts with Shift** (`ui/src/shortcuts/keymap.ts`):
   - `Shortcut.modifier` becomes `'none' | 'shift' | 'primary'`.
   - Rule in `matches`: `'shift'` requires Shift. `'none'` with a named key (`ArrowLeft`, `Delete`; `key.length > 1`) requires no Shift. For a printable character, Shift is ignored, because it is part of the character (`+` is Shift+`=` on many layouts).
   - New actions: `zoomIn` `+`, `zoomOut` `-`, `zoomFit` `0`, `zoomActual` `1`, `panLeft/Right/Up/Down` = Shift+arrows.
   - Tests: the uniqueness test is extended to key + modifier; Shift+ArrowLeft is not `previous`; `+` matches with Shift.
3. **`feat:` zoom domain** in `ui/src/features/viewer/zoom.ts` (pure, with a unit test file):
   - `type Zoom = { kind: 'fit' } | { kind: 'zoomed'; scale: number; center: Point }`, where `center` is a fraction of the photo (0..1).
   - `Geometry` = the photo's own size plus the view's size.
   - `zoomReducer(zoom, action, geometry)` with these actions: `zoomBy(factor, at?)`, `fit`, `actual(at?)`, `toggle(at)`, `panBy(dx, dy)` (pixels), `photoChanged(sameSeries)`.
   - Helpers: `fitScale`, `clampCenter`, `layoutOf(zoom, geometry)` → the size and offset of the image in pixels.
   - Tests: the spot under the pointer stays put when zooming; snap to fit; the 800 % cap; panning clamped at the edges; centring on a small axis; the relative centre kept across photos of different sizes; `photoChanged(false)` resets.
4. **`feat:` viewer zoom** in `usePhotoZoom.ts` (a hook) plus `ZoomBadge.tsx`, connected in `PhotoViewer`/`FullPhoto`:
   - The hook owns `useReducer(zoomReducer)`. It measures the view with a `ResizeObserver` and the photo's own size on `<img>` load.
   - It adds a non-passive `wheel` listener (to `preventDefault` the webview's own Ctrl+wheel page zoom), pointer drag with `setPointerCapture`, and `dblclick`. All of these are cleaned up in effects.
   - Keys are bound through `useShortcuts`; the pan and zoom-out keys are `null` at fit.
   - Same series = the new photo's id is among the current `photo.series` ids. That way a rejection that shows the next shot of the series keeps the zoom.
   - Zoomed images get explicit width/height plus `transform: translate()`; fit keeps today's `object-fit: contain`. CSS: `touch-action: none`, grab/grabbing cursors.
   - `pl.json`: `viewer.zoomFit` („Dopasuj”), `viewer.zoomLevel` (aria label „Powiększenie”).
   - Component tests (Testing Library): the keys show and hide the badge, „Dopasuj” returns to fit, and stepping to another series resets the zoom. The photo's size is faked in a `load` event via `naturalWidth`/`naturalHeight`.
5. **`test(e2e):`** in `ui/e2e/viewer.spec.ts`: serve an SVG photo of 2000×1500. Then check that `1` shows „100 %”; that stepping within the series keeps it while stepping out of the series resets it; that double-click toggles; and that Shift+→ pans without stepping.
6. **`docs:`** describe zoom in the viewer section of `docs/domain/timeline.md`, set slice 13 to 🧪 in the slice file and `docs/ROADMAP.md`, and add `docs/ideas/side-by-side-compare.md`. Note in slice 21 (faces) that it can reuse `zoomReducer`'s `actual(at)` to zoom to a face. Note in slice 23 (macOS) that a pinch in WKWebView arrives as `gesture*` events, not Ctrl+wheel.

Every commit: `cargo` checks are not affected (UI only). Run `npm run lint`, `npm run typecheck -w ui`, `npm test` and `npm run e2e`, then push to `main`.

## Verification

- Unit tests and e2e as above.
- Debug build (`npm run tauri dev`), only when the user is not scanning: zoom into a face with the wheel and with a touchpad pinch, step through the series and check the same spot stays in view; `1` shows sharp real pixels; `0` and „Dopasuj” fit; the next series starts fitted; Shift+arrows pan; the window's own page zoom never kicks in.
- Then pause for the user's test (milestone 🧪).

## Implementation notes (2026-10-30)

Implemented as planned, in `ui/src/features/viewer/` (`zoom.ts`, `usePhotoZoom.ts`, `ZoomBadge.tsx`, `FullPhoto.tsx`) and `ui/src/shortcuts/keymap.ts`. Behaviour is now described in [docs/domain/timeline.md](../domain/timeline.md#zoom). Where it differs from the plan:
- **100 % is in screen pixels**, not CSS pixels: the zoom divides by `devicePixelRatio`, so with Windows display scaling at 150 % one photo pixel is still one screen pixel.
- **Fit enlarges small photos**, as it already did before (CSS `object-fit: contain`); the plan's „not enlarged (as today)” was mistaken about today. For such a photo `1` stays at fit.
- The badge shows the level as `Intl` formats it for `pl-PL`, i.e. „100%” without a space.
- Pan step for Shift+arrows: 120 view pixels.

Tested by the user in the debug build on 2026-10-30: works; released in v0.13.0.
