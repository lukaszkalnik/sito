---
status: ⬜ not started
---

# Slice 59 – Edit series in the viewer

Split off from [slice 14](14-edit-series.md) in its review (2026-11-18); concept and plan when the slice starts.

## Idea

The edits of slice 14 („Podziel serię przed tym zdjęciem”, „Wyjmij z serii”, „Dołącz do poprzedniej/następnej serii”, „Rozdziel serię”, „Przywróć automatyczne grupowanie”) also in the viewer, where the user looks closely at a series: from the bar or the series strip, with keys from `ui/src/shortcuts/keymap.ts`.

## Try

Step through a chained series in the viewer, cut it where the moment changes and take out a stray photo, without going back to the main window; the strip and the grid follow.

## Builds on

- [Slice 14](14-edit-series.md): pinned membership, the `edit_series` command and the pure function listing the possible edits for a photo. This slice only adds the viewer's controls and keys.
  - `ui/src/features/series-edit/editOptions.ts`: `photoEditOptions(arranged, id)` takes the photos in display order, the series ranges and the pinned ids; the viewer can build that from its own page data. `editSeries(edit, gap)` in `series-edit/api.ts`, and the toast texts in `useSeriesMenu.ts` (worth moving into a shared hook when the viewer needs them).
