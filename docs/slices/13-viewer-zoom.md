# Slice 13 – Zoom in the viewer

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

This was asked for in the user's first test (2026-10-27): to choose the best shot, sharpness and faces have to be compared at 100 %, which the fitted view can't show.

## Concept

- **Zoom input:** the mouse wheel and touchpad pinch zoom around the pointer. Keys: `+`/`-`, `0` = fit, `1` = 100 %.
- **Pan:** drag the photo. When zoomed in, the arrow keys step to the previous or next photo as now; panning with keys could use Shift+arrows (to decide).
- **Same detail across a series:** stepping to another photo keeps zoom level and position (relative to the photo), so the same spot can be compared in every shot.
- **Full resolution:** the viewer already loads the original file. At 100 % the webview shows full resolution, with no re-encoding.
- **Zoom indicator:** a small „100 %” badge, plus a reset button.

## Open questions

- Should zoom also be kept when moving on to another series, or reset there?
- Is a side-by-side comparison of two photos at the same zoom needed (a later slice)?
- Shift+arrows for panning, or keep the arrows only for stepping?

## Verification (outline)

- Zoom into a face, step through the series, and the same spot stays in view in every shot.
- 100 % shows real pixels.
- Fit returns to the whole photo.
