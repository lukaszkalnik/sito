---
status: 📝 concept drafted
---

# Slice 62 – Undo series edits

Drafted on 2026-11-20 in the second review of [slice 59](59-edit-series-in-viewer.md), to be done before it; plan when the slice starts.

## Idea

Make the series edits of [slice 14](14-edit-series.md) undoable with „Cofnij” / Ctrl+Z, like rejections. Today edits are not part of „Cofnij” ([Grouping](../domain/grouping.md#series-edited-by-hand)). A wrong edit (a cut at the wrong photo, a photo taken out by mistake, a series ungrouped) then leaves the user confused about where the photo went and how to get the series back: „Przywróć automatyczne grupowanie” is not the inverse of an edit (it gives back the whole edited stretch to automatic grouping, also earlier edits), and „Cofnij” right after an edit undoes the *previous rejection* instead, which is worse. This matters even more once the viewer offers edits with single keys (slice 59: P, W).

## Try

Take a photo out of a series by mistake, press Ctrl+Z: the photo is back in its series. Cut, join and ungroup, then undo them one by one, newest first, mixed with rejections; restart and undo again.

## Builds on

- The action log (`crates/core/src/actions.rs`): rejections are entries undone newest first (`last_undoable`, `mark_undone`), also after a restart.
- Pins (`crates/core/src/series/pins.rs`, `pin_store.rs`): an edit is a `PinChanges` (`set`, `clear`) applied to the `series_pins` table, which holds only the current pins. Undoing needs the pins the touched photos had before the edit.

## Concept (to plan)

- **One history for „Cofnij”**, newest first: rejections and edits in one order, so Ctrl+Z always undoes the last thing the user did. Accepting stays outside it ([Accepting](../domain/accepting.md)).
- **Each edit is logged with what it replaced**: in the same transaction as the pin changes, a log entry with the previous pin (or none) of every photo the edit touched. Undo writes those back. A new numbered migration (e.g. an `action_pins` table next to `action_moves`); applied migrations stay as they are.
- **Edits that no longer fit**: a photo of the edit was rejected, trashed, moved outside Sito or renamed since, or a later edit changed it again. Since undo is newest first, later edits are undone before; open whether an edit over a photo rejected after it is undone (the rejection is newer, so it is undone first) or skipped with a toast, as rejections whose file is gone are.
- **UI**: the toasts of the edits get „Cofnij” (kind `undoable`, as after a rejection); the undo toast says what came back („Cofnięto podział serii”, „<name> wróciło do serii”, …, with proper plural forms where counts appear); the tooltip of „Cofnij” no longer only speaks of rejected photos („Cofnij ostatnią zmianę (Ctrl+Z)” or similar). Both windows: an edit made in the grid can be undone in the viewer and the other way round.
- **Domain docs**: [Grouping](../domain/grouping.md) and [Rejecting](../domain/rejecting.md) (what „Cofnij” covers).

## Related slices

- [Slice 59](59-edit-series-in-viewer.md) builds on it: the viewer's edits are undone with the viewer's „Cofnij” with no extra work.
- [Slice 60](60-select-and-drag-photos.md): a drag of several photos or „Połącz w serię” is one edit, undone in one step.
- [Slice 16](16-show-rejected.md) (show and restore rejected photos) reads the same log.
