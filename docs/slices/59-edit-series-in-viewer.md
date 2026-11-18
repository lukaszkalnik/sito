---
status: ⬜ not started
---

# Slice 59 – Edit series in the viewer

Split off from [slice 14](14-edit-series.md) in its review (2026-11-18); concept and plan when the slice starts.

## Idea

The edits of slice 14 („Podziel serię przed tym zdjęciem”, „Wyjmij z serii”, „Dołącz do poprzedniej/następnej serii”, „Rozdziel serię”, „Przywróć automatyczny podział”) also in the viewer, where the user looks closely at a series: from the bar or the series strip, with keys from `ui/src/shortcuts/keymap.ts`.

## Try

Step through a chained series in the viewer, cut it where the moment changes and take out a stray photo, without going back to the main window; the strip and the grid follow.

## Builds on

- [Slice 14](14-edit-series.md): pinned membership, the `edit_series` command and the pure function listing the possible edits for a photo. This slice only adds the viewer's controls and keys.
