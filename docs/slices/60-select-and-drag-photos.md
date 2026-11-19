---
status: 📝 concept drafted
---

# Slice 60 – Select photos and drag them into series

Drafted on 2026-11-18 while testing [slice 14](14-edit-series.md); plan when the slice starts.

## Idea

Two more ways to fix series by hand in the grid, beyond the context menu of slice 14:

- **Drag & drop:** drag a photo (or several selected photos) onto a series to add them to it, e.g. a single photo that belongs to a series a few photos away, or a photo from one series into another.
- **Selection:** select several photos with the mouse or keyboard and group them into a new series („Połącz w serię”), e.g. single photos of one moment that automatic grouping missed (different cameras, file dates, a gap a bit too long). The selected photos leave the series they were in; those keep the rest, as with „Wyjmij z serii”.

A click on a tile opens the photo in the viewer, and that stays the primary action: selecting must not get in its way, and a user who never selects should not notice the feature until they look for it.

## Try

Select three single photos of one moment, group them into a series; drag a stray photo onto the series it belongs to; drag a photo out of a series onto another one. Check that a plain click still opens the viewer at every step except while selecting.

## Builds on

- [Slice 14](14-edit-series.md): edits are pins, applied with `edit_series`; the menu entries come from `ui/src/features/series-edit/editOptions.ts`. Dropping onto a series is `Edit::Join`, which already accepts any series, not only a neighbouring one (the arrangement shows a series in one piece at its newest photo, so a far-away photo moves there). Grouping a selection needs a new edit (`Edit::Group { photos }`: a new manual series of the photos; the series they come from keep the rest), and joining several photos at once `Edit::Join` with a list.
- [Grouping](../domain/grouping.md#series-edited-by-hand), [Timeline](../domain/timeline.md#editing-series).

## Candidate approaches: entering selection

1. **Modifier clicks (file-manager convention).** Ctrl+click (Cmd+click on macOS) toggles a photo, Shift+click selects a range in display order; a plain click still opens the viewer. Nothing changes for users who never select, and it works without a mode. Weak point: hard to discover; needs a hint (tooltip, a context menu entry „Zaznacz”).
2. **A select circle on hover (Google Photos style).** Hovering a tile shows a small empty circle in a corner; clicking it selects the photo and enters selection mode. In selection mode a plain click on any tile toggles it instead of opening the viewer, a bar replaces the toolbar's right side („3 zaznaczone · Połącz w serię · Anuluj”), and Escape or „Anuluj” leaves the mode. Discoverable, works with a touchpad alone, and the click that opens the viewer is only taken over while the mode is visibly on. Needs a free corner: the pencil of slice 14 sits top-left, format badges top-right, the accepted check bottom-right; bottom-left or moving the pencil.
3. **An explicit mode button** in the bar („Zaznaczanie”). Clear, but one more control in a full bar and one more click every time.
4. **Rubber-band selection** by dragging over empty grid space. Natural on desktops, but the grid has little empty space and dragging starts drag & drop on tiles.

Recommended: 2 with the modifiers of 1 as shortcuts (Ctrl/Cmd+click enters selection mode with that photo, Shift+click extends). Keyboard: a keymap action (e.g. Space on a focused tile, or X) toggles the focused photo and enters the mode; the context menu of slice 14 gains „Zaznacz”.

## Candidate approaches: drag & drop

- **HTML5 drag & drop** on the tile's open button: a click without movement still opens the viewer; dragging past a few pixels starts a drag (of the whole selection if the tile is selected). Drop targets: a series block or header (highlighted while hovered) → join; the drag image shows the count. Needs auto-scroll near the grid's top and bottom edges, as the grid is virtualized and the target may be off screen. WebKit (macOS) and WebView2 (Windows) both support it; check that the webview's own file-drop handling (Tauri's drag-drop events) does not swallow in-page drags.
- **Pointer events with a custom drag layer**: full control over the look and auto-scroll, more code, and keyboard users get the same edits from the menu anyway.
- Possible later: dropping onto empty space between photos to take a photo out of its series (today: „Wyjmij z serii”).

## Open questions

- In „Serie” mode single photos are hidden: selecting singles to group needs „Wszystkie zdjęcia”; should grouping switch the view, or should the bar say so?
- Should selection also offer other batch actions later (reject, accept several photos at once)? Keep the selection state generic (a set of ids in the timeline feature) so they can be added.
- What happens to a selection when the timeline changes (a refresh, an edit in another window): keep the ids still shown, drop the rest.
- [Slice 59](59-edit-series-in-viewer.md) (edits in the viewer) is independent; selection stays a grid feature. It plans a shared `useEditSeries` hook (edit + toasts) to reuse here, and takes the keys P and W.
