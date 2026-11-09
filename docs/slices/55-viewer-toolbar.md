---
status: 📝 concept drafted
---

# Slice 55 – Viewer bar on small screens

Since 2026-11-09; plan when the slice starts.

## Idea

Asked by the user on 2026-11-09, while testing [slice 48](48-compact-toolbar.md) on a MacBook: give the viewer bar ([Viewer](../domain/timeline.md#viewer)) the same clean-up that slice 48 gave the main window bar, so it stays readable and clear in a narrow window.

Found while testing (screenshot of a 1920 × 1080 Takeout photo): the bar shows name, date, resolution, file size, „data z Google Takeout” and the photo's folder in one line, and the folder is cut to „Zdjęcia Googl…”. The cut-off text has no tooltip and no hint that it is a folder, so the user could not tell what it was.

In a narrower viewer window the facts even ran under „Zaakceptuj” and „Odrzuć”, drawn over them (the info line never shrank or clipped).

## Fixed ahead of planning (2026-11-09)

- The overlap: the info line now clips at its end (`overflow: hidden` on `.viewer-info`), and the folder gives way first, with an ellipsis, while the other facts keep their width (`PhotoViewer.css`). Regression test in `ui/e2e/viewer.spec.ts` („keeps the photo info clear of the buttons in a narrow window”). The facts cut off at the edge are still not reachable; that is part of this slice's plan.

## To decide when planning

- **Photo facts:** which facts stay in the line and in what order, what gets a label or an icon (the folder especially), and full texts as tooltips where they are cut. Possibly two lines (name and folder above, date and facts below), like the path and summary in the main window bar.
- **Narrow windows:** what moves or shrinks below a breakpoint, measured like slice 48 did (`ui/e2e/toolbar.spec.ts` checks for overlaps, horizontal overflow, readable widths and a fixed height per width).
- **Buttons:** „Zaakceptuj”, „Odrzuć”, „Cofnij” (already an icon button, slice 48) and „Zamknij”. Possibly icons with tooltips for some of them, keeping „Odrzuć” clearly set apart from „Cofnij” ([Viewer](../domain/timeline.md#viewer)).
- **Look:** the viewer is always dark. Its buttons keep their own style (`viewer-close` in `PhotoViewer.css`) today; the shared bar look of slice 48 (`ui/src/controls/controls.css`, `--control-border`, accent-coloured icon buttons) could get a dark variant. This also covers what [slice 23](23-macos-polish.md) leaves for the viewer bar.

## Builds on

- [Slice 48](48-compact-toolbar.md): `Field` (caption above a control), `controls.css`, the icon buttons (`UndoButton`, `UndoIcon`, `ChooseFolderButton`) and the toolbar e2e checks.
