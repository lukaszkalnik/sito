---
status: 📝 concept drafted
---

# Slice 48 – Main window bar on small screens

Since 2026-11-07; plan in plan mode when the slice starts.

## Idea

Asked by the user on 2026-11-07, with a screenshot of the main window on a MacBook screen: the bar above the grid ([Main window bar](../domain/timeline.md#main-window-bar)) gets crowded on a small screen. The folder path and the summary are cut to „/Users/lukasz/Pictures/Sk…” and „Znaleziono 1461 zdjęć. 24…”, while the controls next to them take most of the width with their inline labels („Odstęp w serii”, „Odrzucone zdjęcia do:”).

Clean the bar up so it stays readable and usable in a narrow window.

## Candidate approaches

- **Labels in a row above the controls:** a small caption line above each control group („Odstęp w serii” over the slider, „Odrzucone zdjęcia do:” over the dropdown) instead of a label next to it, so each group is only as wide as its control.
- **Tooltips instead of labels** where the control explains itself (e.g. the folder icon before the rejection target, or the „Serie / Wszystkie zdjęcia” switch); the label stays as the `aria-label` from `pl.json`, so screen readers still name the control.
- Possibly a mix: captions for the slider and the rejection target, tooltips elsewhere; and give the folder path and summary more of the width.

## To check when planning

- The bar must keep a steady height ([slice 37](37-steady-top-bar.md)): a caption row is a fixed extra line, not one that appears and disappears.
- Which window widths to support (smallest MacBook screen, a half-screen window on Windows) and what happens below that (wrap into a second row of controls vs. hide less-used ones).
- Keyboard access and visible focus are unchanged; icon-only controls keep an `aria-label`.
- Tests: Vitest checks controls are still found by role/label; a Playwright check at a narrow viewport that nothing overlaps and the path/summary keep a minimum width.
- Docs: `docs/domain/timeline.md` (Main window bar).

## Related

- [Slice 23](23-macos-polish.md) (macOS polish) notes that „Wybierz inny folder” does not look like a native macOS button; if this slice restyles the bar's buttons anyway, consider doing that part here.
