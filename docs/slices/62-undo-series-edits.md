---
status: 📝 planned
---

# Slice 62 – Undo series edits

Drafted on 2026-11-20 in the second review of [slice 59](59-edit-series-in-viewer.md), to be done before it. Planned on 2026-11-20; waiting for the user's approval.

## Idea

Make the series edits of [slice 14](14-edit-series.md) undoable with „Cofnij” / Ctrl+Z, like rejections. Today edits are not part of „Cofnij” ([Grouping](../domain/grouping.md#series-edited-by-hand)). A wrong edit (a cut at the wrong photo, a photo taken out by mistake, a series ungrouped) then leaves the user confused about where the photo went and how to get the series back: „Przywróć automatyczne grupowanie” is not the inverse of an edit (it gives back the whole edited stretch to automatic grouping, also earlier edits), and „Cofnij” right after an edit undoes the *previous rejection* instead, which is worse. This matters even more once the viewer offers edits with single keys (slice 59: P, W).

## Try

Take a photo out of a series by mistake, press Ctrl+Z: the photo is back in its series. Cut, join and ungroup, then undo them one by one, newest first, mixed with rejections; restart and undo again.

## Builds on

- The action log (`crates/core/src/actions.rs`): rejections are entries undone newest first (`last_undoable`, `mark_undone`), also after a restart.
- Pins (`crates/core/src/series/pins.rs`, `pin_store.rs`): an edit is a `PinChanges` (`set`, `clear`) applied to the `series_pins` table, which holds only the current pins. Undoing needs the pins the touched photos had before the edit.

## Concept

- **One history for „Cofnij”**, newest first: rejections and edits in one order, so Ctrl+Z always undoes the last thing the user did. Accepting stays outside it ([Accepting](../domain/accepting.md)).
- **Each edit is logged with what it replaced**: in the same transaction as the pin changes, a log entry with the previous pin (or none) of every photo the edit touched. Undo writes those back. A new numbered migration (e.g. an `action_pins` table next to `action_moves`); applied migrations stay as they are.
- **Edits that no longer fit**: a photo of the edit was rejected, trashed, moved outside Sito or renamed since, or a later edit changed it again. Since undo is newest first, later edits are undone before; open whether an edit over a photo rejected after it is undone (the rejection is newer, so it is undone first) or skipped with a toast, as rejections whose file is gone are.
- **UI**: the toasts of the edits get „Cofnij” (kind `undoable`, as after a rejection); the undo toast says what came back („Cofnięto podział serii”, „<name> wróciło do serii”, …, with proper plural forms where counts appear); the tooltip of „Cofnij” no longer only speaks of rejected photos („Cofnij ostatnią zmianę (Ctrl+Z)” or similar). Both windows: an edit made in the grid can be undone in the viewer and the other way round.
- **Domain docs**: [Grouping](../domain/grouping.md) and [Rejecting](../domain/rejecting.md) (what „Cofnij” covers).

## Plan

### Behaviour

- **One history for „Cofnij”** in the open folder, newest first: rejections and the five edits of slice 14 („Podziel serię przed tym zdjęciem”, „Wyjmij z serii”, „Dołącz do poprzedniej/następnej serii”, „Rozdziel serię”, „Przywróć automatyczne grupowanie”) in the order the user made them. Ctrl+Z, the bar's „Cofnij” and the toast's „Cofnij” undo whichever is newest. Accepting stays outside. Undoing „Przywróć automatyczne grupowanie” brings the edited stretch back as it was.
- **Undo puts back the pins the edit replaced**, for every photo it touched (also the needless pins it cleared, and photos gone for now). The series then look as they did right before the edit, at the current gap: if the gap changed since, the unpinned photos follow the new gap, as they would have before the edit.
- **An edit that changed no pin** is not logged (nothing to undo; Ctrl+Z then goes on with the action before). With the current edit options this does not happen, but slice 60's drags may produce one.
- **Edits that no longer fit do not arise:** undo is strictly newest first, so a later edit or rejection of the same photos is undone before (answering the open question: a rejection made after an edit is undone first, then the edit; no skipping). What happens outside the history is handled so the edit still undoes cleanly:
  - A photo moved or renamed outside Sito, or restored by „Cofnij” under a new name: its pin follows it today (`move_pins`); the logged previous pins follow it the same way, so undoing the edit reaches it at its new path.
  - A photo gone (trashed, deleted outside Sito, or a rejection whose undo failed): its pin is put back as well; it is not shown, and comes back into its series when it returns, as today.
  - On macOS rejections into the trash are skipped by „Cofnij” (not undoable there): an edit behind one is undone, as a folder rejection behind one is today.
  - **Nested folders** (accepted limitation, as for rejections): an action is offered in the folder its photo is in, also when opened as a subfolder. An edit made with the parent folder open that touched photos of a subfolder too is undone from the subfolder only if its photo is there; undoing an older edit from the subfolder past it still gives a valid grouping, just not the exact one from before. Rare enough not to guard against.
- **Ids of manual series are never reused** while a logged edit may put one back: a new series takes an id above every id in the pins and in the log's previous pins. Otherwise undoing an edit in one folder could put back an id that a later edit in another folder reused, and opening their common parent would merge the two series.
- **Toasts**
  - After an edit: kind `undoable` (with „Cofnij”), texts as today („Podzielono serię”, „Wyjęto <name> z serii”, …).
  - After undoing an edit: „Cofnięto podział serii”, „Cofnięto wyjęcie <name> z serii”, „Cofnięto dołączenie <name> do serii”, „Cofnięto rozdzielenie serii”, „Cofnięto przywrócenie automatycznego grupowania”. No counts appear, so no plural forms yet (slice 60 adds counts with `Intl.PluralRules` / i18next plurals).
  - A failed undo of an edit (the log cannot be written): „Nie udało się cofnąć zmiany serii.”
  - Tooltip of „Cofnij” (main window and viewer): „Cofnij ostatnią zmianę (Ctrl+Z)”; the rejection texts stay.
- **Reveal: only photos that visibly changed flash.** An edit's changes can reach far beyond the photo the user clicked (undoing „Przywróć automatyczne grupowanie” brings back a whole edited stretch, e.g. both parts of a cut and a photo taken out; undoing a join or a cut changes the series on both sides), so the highlight covers every photo that changed, but no photo that looks as before.
  - **A photo changed visibly** when it went from a series to single or back, moved to another series, or its edited mark changed (the pinned mark of a single photo, the edited mark of its series' header). Which series counts as „the same” before and after: each series after the undo is matched to the series before it that shares the most photos with it (one to one, larger overlaps first); a photo staying in its matched series did not move. So undoing „Wyjmij z serii” flashes only the photo that came back, not the rest of its series; undoing a cut flashes the photos of the smaller part, which joined the larger one; undoing „Rozdziel serię” flashes the whole series. Photos whose pin only changed in the log (needless pins) and still look the same do not flash.
  - Worked out in the backend from the grouping before and after the undo at the current gap (pure function in core, over the touched photos and the photos of every series they are in before or after), so the undo command takes the gap like `edit_series`. Gone photos are skipped.
  - **Grid:** scrolls to the newest changed photo and highlights every changed one (the pulses of a photo brought back by undoing a rejection). Nothing changed visibly (e.g. only marks of photos gone for now): no reveal.
  - **Viewer:** keeps showing its photo (switching it would be unexpected); only the strip follows the new series of that photo and highlights its changed photos with the same pulses. If the shown photo is no longer on the timeline, the viewer behaves as today when its photo goes. Undoing a rejection is unchanged (the viewer shows the photo that came back).
  - Both windows: an edit made in the grid is undone from the viewer and the other way round (the viewer has no edits until slice 59).

### Core (`crates/core`)

- **Migration 6** (`actions.rs`): `action_pins (action_id INTEGER NOT NULL REFERENCES actions(id), path TEXT NOT NULL, pinned INTEGER NOT NULL, series INTEGER NULL, PRIMARY KEY (action_id, path))`: for each photo an edit touched, the pin it had before (`pinned = 0`: none; `pinned = 1, series NULL`: single; else its manual series). Applied migrations stay as they are; a v5 log keeps its pins and rejections.
- **Edit actions** in `actions`: kinds `series_cut`, `series_remove`, `series_join`, `series_ungroup`, `series_restore`; `from_path` = `to_path` = the path of the edit's photo (which decides the folder it is offered in, as for rejections). A new `EditKind` enum in `series/edit.rs` (`Edit::kind()`), mapped to and from these texts with an exhaustive `match`.
- **`series/edit_log.rs`** (new, `impl ActionLog`): `record_edit(kind, photo, &PinChanges) -> Result<Option<i64>>` reads the current pins of the touched paths, writes the action and its previous pins, and applies the changes, all in one transaction (replaces the bare `apply_pin_changes` call of the edit command; `apply_pin_changes` stays for tests or goes if unused). `undo_edit(id) -> Result<PinChanges>` puts the previous pins back and marks the action undone in one transaction, and returns the changes it made so the open scan applies them.
- **`last_undoable`** returns `Option<Undoable>`, an enum `Reject(RejectAction)` / `Edit(EditAction { id, kind, photo })`, the newest not undone of either inside the folder; `Rejections::FolderOnly` still only leaves out trash moves. Unknown kinds (old `accept` rows) stay ignored.
- `move_pins` also moves the previous pins of edits not undone (`UPDATE OR IGNORE action_pins … WHERE path = from`), in the same transaction. `next_series_id` takes the highest of `series_pins` and `action_pins`.
- `actions.rs` is near 400 lines with this: the shared row reading stays there; edit-specific reading goes into `edit_log.rs`.

### Tauri (`src-tauri`)

- New `src-tauri/src/undo.rs`: the `undo` command (replacing `undo_reject`) and `get_undo_state`, moved out of `reject.rs`. `undo` takes the newest `Undoable` and hands it to `reject::restore_rejection` (as today) or `series::edit::undo_edit` (applies the returned changes to the scan, announces `timeline-changed`).
- `UndoneDto` becomes a tagged union: `{ kind: "rejection", id, name, restoredName }` / `{ kind: "edit", edit: "cut" | "remove" | "join" | "ungroup" | "restore", name, changed }` (`changed`: ids of the visibly changed photos, newest first; ids only, so the payload stays small even for a long stretch). `UndoStateDto` keeps its three states, now counting edits as `available`.
- `edit_series` logs through `record_edit`.

### UI (`ui`)

- `reject/api.ts`: `undo()` invokes `undo`; `Undone` becomes the discriminated union above, validated in the data layer. `useUndo` builds the toast text from it (exhaustive `switch` with `assertNever`); `onUndone` gets the outcome: the grid reveals the restored photo or the changed photos, the viewer shows a restored photo or highlights changed photos in its strip. The grid's `Reveal` gets a set of ids to highlight besides the one it scrolls to (`reveal.ts`, `TimelineTile` highlight), and `SeriesStrip` a set of ids to pulse, so both are generic for slice 60's multi-photo edits. `undo(gap)` passes the window's gap.
- `useSeriesMenu`: toast kind `undoable`.
- `pl.json`: `undo.hint` changes; new `undo.edit.cut|remove|join|ungroup|restore` and `undo.editFailed`.
- `ui/e2e/tauri-mock.ts`: one history of rejections and edits; `edit_series` keeps a snapshot of the mocked grouping so `undo` can put it back.

### Tests

- **Core, changed photos** (pure function): series to single and back, a move to another series, a mark change alone, needless pins (no flash), a cut undone (smaller part), ungroup undone (all), remove undone (one photo), join undone (one photo), ties in the matching, gone photos, nothing changed.
- **Core** (`edit_log.rs`, `actions.rs`): previous pins of every kind (none, single, other series) are logged and put back exactly; each edit kind round-trips through its text; an edit with no changes is not logged; `last_undoable` returns the newest of rejections and edits in any interleaving, skips undone ones, keeps to the folder, and with `FolderOnly` skips a newer trash move to an edit; `move_pins` moves the previous pins of open edits but not of undone ones; `next_series_id` stays above logged ids; a v5 log migrates with its pins and rejections intact.
- **Tauri** (`series/edit.rs`, `undo.rs`, real files in temp dirs, the EXIF fixtures): every edit (cut, remove, join to the previous and to the next series, ungroup, restore) undone gives back the grouping before it; sequences undone newest first: join then remove, cut then join back, ungroup then restore, two edits of different series; edits mixed with rejections (edit, reject a photo of the series, undo twice; reject, edit, undo twice); undo after a gap change; undo after a restart (a fresh `ActionsState` on the same database); a photo restored under a new name between the edit and its undo; nothing left to undo returns `None` and the state `none`.
- **UI** (Vitest): `useUndo` toast for each edit kind and for a failed edit undo, the outcome passed on; the grid highlights every changed photo and scrolls to the newest (`useGridScroll` / `TimelineGrid` tests); the viewer keeps its photo and pulses the changed photos of its strip; `useSeriesMenu` shows an undoable toast; `UndoButton` hint. **Playwright** (`series-edit.spec.ts`): take a photo out, undo from the toast; cut and ungroup, then Ctrl+Z twice, newest first, mixed with a rejection; undo of a grid edit from the viewer; undoing „Przywróć automatyczne grupowanie” of a cut series highlights the photos of the smaller part only; the viewer keeps its photo when a grid edit is undone from it.
- No new real-app spec: the files and the log are covered by the Rust tests on real files, the windows by the mocked suite.
- **Real app:** this session runs in a Linux cloud container without the user's machine, so the computer-use pass (edit and undo in both windows, restart, check the action log, take back test edits) is left to the user's test, unless a machine is attached.

### Domain docs

- [Grouping](../domain/grouping.md#series-edited-by-hand): edits are undone with „Cofnij”; drop „Edits are not part of „Cofnij””.
- [Rejecting](../domain/rejecting.md): „Cofnij” undoes rejections and series edits in one history.
- [Timeline](../domain/timeline.md): the toasts and the tooltip of „Cofnij”.

### Commits

1. `feat: log series edits with the pins they replace` (core: migration 6, `record_edit`, `undo_edit`, `last_undoable`, `move_pins`, `next_series_id`, tests).
2. `feat: undo series edits with Cofnij` (Tauri `undo.rs`, changed photos, UI, translations, mocked e2e, domain docs, slice and roadmap to 🧪).

## Related slices

- [Slice 59](59-edit-series-in-viewer.md) builds on it: the viewer's edits are undone with the viewer's „Cofnij” with no extra work.
- [Slice 60](60-select-and-drag-photos.md): a drag of several photos or „Połącz w serię” is one edit, undone in one step.
- [Slice 16](16-show-rejected.md) (show and restore rejected photos) reads the same log.
