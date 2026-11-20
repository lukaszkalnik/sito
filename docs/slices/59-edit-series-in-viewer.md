---
status: 📝 planned
---

# Slice 59 – Edit series in the viewer

Split off from [slice 14](14-edit-series.md) in its review (2026-11-18). Plan drafted on 2026-11-19, narrowed in the user's first review the same day (no joins in the viewer) and revised in the second review on 2026-11-20 (stay in the series after „Wyjmij z serii”, nothing to edit on a series of one, edits undoable first in [slice 62](62-undo-series-edits.md)); „Przywróć automatyczne grupowanie” added on the user's request on 2026-11-20 („kind of undo in another way”); waiting for the user's approval.

## Idea

Edits of slice 14 also in the viewer, where the user looks closely at a series: from the bar, the photo or the series strip, with keys from `ui/src/shortcuts/keymap.ts`.

## Try

Step through a chained series in the viewer, cut it where the moment changes and take out a stray photo, without going back to the main window; the strip and the grid follow. Undo a wrong edit with „Cofnij”, or give the series back to automatic grouping with „Przywróć automatyczne grupowanie”.

## Builds on

- [Slice 14](14-edit-series.md): pinned membership, the `edit_series` command and the pure function listing the possible edits for a photo. This slice only adds the viewer's controls and keys.
  - `ui/src/features/series-edit/editOptions.ts`: `photoEditOptions(arranged, id)` takes the photos in display order, the series ranges and the pinned ids. `editSeries(edit, gap)` in `series-edit/api.ts`, and the toast texts in `useSeriesMenu.ts` (moved into a shared hook here).
  - `SeriesMenu` (positioned at window pixels, keyboard usable, the WebKit blur fix from testing) can be shown in the viewer as it is, once the viewer's keys are kept out while it is open (see [Keys while the menu is open](#keys-while-the-menu-is-open)).
- [Slice 62](62-undo-series-edits.md) (done before this slice): edits are undone with „Cofnij” / Ctrl+Z like rejections, and their toasts offer „Cofnij”. The viewer's „Cofnij” and its toast then cover the edits made here with no extra work.
- How the viewer works today: [Timeline and viewer](../domain/timeline.md#viewer); edits: [Grouping](../domain/grouping.md#series-edited-by-hand), [Editing series](../domain/timeline.md#editing-series).

## Concept

### Only what the strip shows

**The viewer edits only the photos the user sees in it: the current series (the strip)** (decision of the first review, 2026-11-19). An edit there must not reach photos outside the strip, which the user cannot see from the viewer. So the viewer offers:

- **„Podziel serię przed tym zdjęciem”** and **„Wyjmij z serii”**, on the shown photo or on any photo of the strip, under the same rules as the grid (a cut only in the middle of a series of three or more, taking out only in a series of three or more);
- **„Rozdziel serię”**, which only splits the series of the strip into its own photos (confirmed in the second review);
- **„Przywróć automatyczne grupowanie”** (added on the user's request, 2026-11-20): on a photo of an edited series (the strip shows the pencil) and on a single photo pinned by an edit (the bar shows the pinned pencil), exactly where the grid offers it. It is the other way back from an edit besides „Cofnij”: it gives the whole stretch of edited photos around the photo back to automatic grouping ([Grouping](../domain/grouping.md#series-edited-by-hand)).

**The exception to „only what the strip shows”:** restoring is the one edit that reaches photos outside the strip, by design: photos taken out of the series earlier and the other part of a cut come back, and at the current gap the series may join its neighbours. That is what the user wants from it (getting the series back), and the result is in view right after: the strip then shows the photo's new series, with the photos that came back in it pulsing (see [What the viewer shows after an edit](#what-the-viewer-shows-after-an-edit)). „Cofnij” takes it back as any edit.

It does **not** offer „Dołącz do poprzedniej/następnej serii”: it moves the photo into a series outside the strip, which the viewer does not show, so the user would join blind. Joins stay in the grid.

**An unedited single photo and an unedited series of one have nothing to edit in the viewer.** A series of one is effectively a single photo already; the grid still offers „Rozdziel serię” on it, but whether a series of one should be shown as a series at all is decided in [slice 63](63-series-of-one.md), so the viewer does not add „Rozdziel serię” for it now. An *edited* series of one and a pinned single photo offer „Przywróć automatyczne grupowanie” only.

This also keeps the data simple: the viewer's page already holds the strip's photos in display order (newest first, as the grid), which is all `photoEditOptions` needs for these four edits; restoring is decided by the photo alone (its series edited, or the photo pinned), and the backend works out the stretch it reaches. A pure function builds a small `Arranged` from the page (the strip's photos as entries, the series as one range with `edited` from the page; for a single photo, the photo alone and no series, in `pinnedIds` if the page says it is pinned), runs `photoEditOptions` unchanged and keeps only the viewer's edits (`cut`, `remove`, `ungroup`, `restore`), dropping `ungroup` for a series of one. So the grid and the viewer still share one set of rules. No neighbours outside the series are needed.

### Controls

1. **A bar button with a menu**: an icon-only „Edytuj serię” button (pencil, tooltip „Edytuj serię (Shift+F10)”) between the photo's info and „Zaakceptuj”, opening the `SeriesMenu` below it with the edits of the shown photo. Visible, so the feature is found; disabled (not hidden, so the bar does not shift) when the photo has no edit to offer (an unedited single photo or series of one). Icon only, as the bar is full; the narrow-window threshold at 780 px is checked again.
2. **Context menus, also on the strip**: right-click on the photo opens the menu of the shown photo; **right-click on a strip thumbnail opens the menu of *that* photo** („Podziel serię przed tym zdjęciem”, „Wyjmij z serii”, „Rozdziel serię”, and „Przywróć automatyczne grupowanie” where it fits), without stepping to it, as a right-click on a grid tile does. Shift+F10 / the Menu key open the menu of the focused strip thumbnail, or of the shown photo when nothing else is focused (the viewer's focus is usually on the body, where the grid's shared Shift+F10 handler does nothing, so the viewer has its own handler for this case).
3. **Direct keys for the frequent edits**: the Try is a stepping flow ("step, step, cut, step, take out"), where opening a menu each time is slow. **P** („Podziel serię przed tym zdjęciem”) and **W** („Wyjmij z serii”) on the shown photo, free in the keymap and not the X/Space that [slice 60](60-select-and-drag-photos.md) considers for selecting. Both act once per press (`ONCE_PER_PRESS`): a held key must not take out photo after photo. A key whose edit the photo does not offer (P on the first photo of a series, W on a single photo) does nothing, like ← on the first photo; Shift+P / Shift+W act like P / W (Shift is ignored for printable keys). „Rozdziel serię” and „Przywróć automatyczne grupowanie” stay in the menu (rarer, and wider in effect, so no single key). The menu shows the key next to the two entries.

(A strip toolbar with edit buttons around the strip was considered and left out: it crowds the strip.)

### Keys while the menu is open

Every `useShortcuts` call listens on the window on its own, with no priority between them. In the grid that did not matter, but the viewer binds many keys: with the menu open, Escape would close the menu **and the viewer window**, ←/→ would step, Delete would reject, A accept, P/W edit behind the menu. So while the menu is open, the viewer passes `null` for its own handlers (previous, next, close, reject, accept, undo, zoom and pan, P, W) and only the menu's keys (Escape, ↑/↓) act. Escape then closes only the menu and focus goes back to the opener (the bar button, the strip thumbnail, or the body).

### What the viewer shows after an edit

The viewer refetches its page on the timeline-changed event (as after a rejection; its page cache is keyed by the timeline revision), so the strip, previous/next and the pencil follow, and so does the grid (its frame stays on the shown photo). **The series stays the focus of editing:**

- **After „Wyjmij z serii” on the shown photo the viewer stays in the series**: it shows the photo of the strip after the taken-out one (in display order), or the one before it when it was the last. So "W, W" takes out two photos in a row and the strip stays in view. The taken-out photo is a single photo now, reached with ←/→ in „Wszystkie” (in „Serie” it is outside the view, as any single photo).
- After „Podziel serię przed tym zdjęciem” the shown photo stays shown and the strip shows its new series (it and the photos after it). The photos before it are the previous series now, reached with ← as usual.
- An edit from a strip thumbnail's menu keeps the shown photo; if the shown photo is no longer in the edited photo's series (the cut put it into the other part), the strip shows the shown photo's series. Taking out a thumbnail other than the shown one only removes it from the strip.
- After „Rozdziel serię” no series is left: the shown photo stays shown as a single photo and the strip goes away.
- **After „Przywróć automatyczne grupowanie”** the shown photo stays shown and the strip shows its series as automatic grouping makes it at the current gap: it may be longer than before (photos taken out earlier, the other part of a cut, a neighbouring series that joins), the same, or gone (the photo is a single photo at this gap). The strip pulses the photos that look different now and are in it, as after „Cofnij” ([slice 62](62-undo-series-edits.md)), so the user sees what came back. For this `edit_series` returns the visibly changed photos (`visibly_changed`, as the undo of an edit already does). The grid does not pulse after an edit today, and this slice leaves it so.
- A toast confirms each edit with the texts of slice 14 („Podzielono serię”, „Wyjęto <name> z serii”, „Rozdzielono serię”, „Przywrócono automatyczne grupowanie”) and, from slice 62, „Cofnij”; errors as in the grid. An edit made while the main window's gap is just changing (it waits for a pause in typing) can no longer fit; it then fails with „Nie można zmienić serii – zmieniła się w międzyczasie.”, never edits the wrong photos.
- **Marks:** when the series is edited, the strip starts with the pencil (`EditedIcon`, „Seria edytowana ręcznie – nie zmienia się z odstępem”); a photo pinned as a single photo shows the pinned pencil after its name in the bar („Wyjęte z serii ręcznie – nie łączy się w serię z odstępem”), so the user sees from the viewer what was edited and knows where to restore it (in the grid).
- **Dark look:** the viewer is always dark and sets the dark values of the shared tokens it uses (`--text`, `--bg`, `--control-border`, `--accent` in `PhotoViewer.css`), but `SeriesMenu.css` and the edit marks also use `--text-h`, `--series-bg` and `--series-text`. Without their dark values the menu would show near-black text on the viewer's dark background on a light system, so the viewer adds them (the dark values from `index.css`).

## Plan

### Steps

1. **Tauri** (`src-tauri/src/viewer/page.rs`): `Page` gains `series_edited: bool` (the photo's series is `SeriesKind::Manual`; feeds both the strip's pencil and `edited` of the strip's `Series` in step 3) and `pinned: bool` (the photo's position is in `Grouping::pinned_singles`, which is sorted, so a `binary_search`), for the marks and for offering „Przywróć automatyczne grupowanie”. `EditedDto` gains `changed: Vec<usize>` (the visibly changed photos, newest first, worked out as in `undo_edit`; a shared helper instead of a copy), and `Edited` in `series-edit/api.ts` validates it. Unit tests of `changed` for each edit kind, a restore that brings back a taken-out photo and both parts of a cut, a restore that joins a neighbouring series at the gap. Unit tests of the page: an unedited series, an edited series, a pinned single, an unpinned single, a pinned single after a rejection of another photo and after its undo, an edited series after a gap change.
2. **UI, refactor** (own `refactor:` commit first): `ui/src/features/series-edit/useEditSeries.ts`, holding the toast texts and error mapping now in `useSeriesMenu` (and slice 62's „Cofnij” on the toast), returning `edit(option)`; `useSeriesMenu` uses it. The grid's Shift+F10 handler (`openMenuOfFocused`) stays in `useSeriesMenu`. Existing tests stay green.
3. **UI, domain** (`ui/src/features/viewer/viewerEditOptions.ts`, new):
   - `viewerEditOptions(photo: ViewerPhoto, id: number): EditOption[]`, building the strip's `Arranged` (entries newest first, one `Series` with `label: plain` and `edited` from the page) and filtering `photoEditOptions` to `cut`, `remove`, `ungroup`, `restore`, without `ungroup` for a series of one.
   - `afterRemove(strip, id): number | null`: the photo to show after the shown photo `id` is taken out (the next strip photo, else the previous one).
   - Vitest over the permutations: series of 1, 2, 3, 4 and many; first, middle and last member; the shown photo and another strip photo; a single photo (pinned or not); an edited series (restore offered on every member), an edited series of one and a pinned single photo (restore only), an unedited series (no restore); never a join, also where the grid would offer one; `afterRemove` for first, middle and last.
4. **UI, viewer controls** (`ui/src/features/viewer/`):
   - `EditSeriesButton` in `ViewerBar` (icon-only, `aria-label`/tooltip from `pl.json`, disabled without options).
   - Context menus on the photo and on the strip thumbnails, a `useViewerSeriesMenu` hook (opens `SeriesMenu` for a photo id at a position; Shift+F10 / Menu key on the focused thumbnail, or the shown photo when the body has focus; focus back to the opener on Escape).
   - The viewer's own shortcuts are `null` while the menu is open (see [Keys while the menu is open](#keys-while-the-menu-is-open)).
   - Keymap actions `cutSeries` (P) and `removeFromSeries` (W), both in `ONCE_PER_PRESS`, bound with `useShortcuts` in the viewer; the menu shows their keys.
   - After „Wyjmij z serii” on the shown photo, show `afterRemove`'s photo; after „Przywróć automatyczne grupowanie”, pulse the returned changed photos that are in the strip (the pulse of slice 62).
   - Strip pencil and bar badge; the dark values of `--text-h`, `--series-bg`, `--series-text` on `.viewer`.
   - `pl.json` strings („Edytuj serię”, „Edytuj serię (Shift+F10)”). Check the narrow-bar threshold with the new button and adjust it.
5. **Tests**:
   - Vitest: the button (disabled for a single photo and a series of one), the strip's context menu (menu of another photo than the shown one), P/W and Shift+P/Shift+W (including no-op cases and held keys), the shown photo after W (first, middle, last), restore from the menu (edited series, pinned single photo; the strip pulses the photos that came back), toasts; with the menu open, Escape closes only the menu and ←, →, Delete, A, P, W do nothing.
   - Playwright (mocked, `ui/e2e/viewer-series-edit.spec.ts`): cut and take out from the viewer with keys, a cut and a take-out from a strip thumbnail's menu, „Rozdziel serię”, „Przywróć automatyczne grupowanie” after a take-out and after a cut (the taken-out photo / the other part back in the strip), undoing an edit and a restore with „Cofnij”, the strip and the grid following; in both view modes; the menu readable with the light system theme (`colorScheme: 'light'`).
   - No real-app spec: nothing here needs the real windows beyond what `follow-viewer` covers.
6. **Testing in the real app** (debug bundle, computer use; every test edit taken back afterwards with „Cofnij” or „Przywróć automatyczne grupowanie” in the grid, and the action log checked): cut in the middle of a long series, cut again in the new part, take out the first/middle/last photo (the viewer stays in the series), W W W down to two photos (only „Rozdziel serię” left), cut and take out from a strip thumbnail other than the shown one, ungroup, restore from an edited series, from a pinned single photo and from a strip thumbnail (photos taken out earlier come back, a neighbouring series joins at a large gap), undo each of them, P on a first photo and W on a single photo (nothing happens), series of 1 (button disabled) and 2 (only „Rozdziel serię”), Escape with the menu open (viewer stays open), both view modes, light and dark system theme, an edit right after typing a new gap in the main window, a gap change while the viewer is open on an edited series, an edit in the grid while the viewer shows the same series (and the other way round), a narrow viewer window, restart.
7. **Docs**: [Timeline](../domain/timeline.md) (Viewer, Editing series: the viewer's edits, controls and keys, what is shown after an edit (also the pulse after restoring), why joins stay in the grid); this file and the roadmap.

### Open questions for the review

All answered in the second review (2026-11-20):

1. „Rozdziel serię” in the viewer: yes.
2. Direct keys P and W: yes.
3. After „Wyjmij z serii”: stay in the series (show its next photo), not on the taken-out photo.
4. The bar button: pencil icon only.

Added after the second review (2026-11-20, the user): „Przywróć automatyczne grupowanie” in the viewer too, as the other way back from an edit.

Open for the review:

5. Restoring reaches photos outside the strip (taken out earlier, the other part of a cut, a neighbour joining at the gap). The plan accepts that and makes it visible by pulsing what came back in the strip. Fine, or should the viewer ask first when the restore reaches photos outside the strip?

## Related slices

- [Slice 62](62-undo-series-edits.md) makes edits undoable; done first.
- [Slice 63](63-series-of-one.md) decides whether a series of one is shown as a series; if it stays one, the viewer may offer „Rozdziel serię” on it then.
- [Slice 60](60-select-and-drag-photos.md) (selection, drag & drop) stays a grid feature; it can reuse `useEditSeries` for its edits and toasts. Keys taken here: P, W.
