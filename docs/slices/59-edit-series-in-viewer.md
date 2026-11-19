---
status: 📝 planned
---

# Slice 59 – Edit series in the viewer

Split off from [slice 14](14-edit-series.md) in its review (2026-11-18). Plan drafted on 2026-11-19 and narrowed in the user's first review the same day (no joins in the viewer); waiting for the user's approval.

## Idea

Edits of slice 14 also in the viewer, where the user looks closely at a series: from the bar or the series strip, with keys from `ui/src/shortcuts/keymap.ts`.

## Try

Step through a chained series in the viewer, cut it where the moment changes and take out a stray photo, without going back to the main window; the strip and the grid follow.

## Builds on

- [Slice 14](14-edit-series.md): pinned membership, the `edit_series` command and the pure function listing the possible edits for a photo. This slice only adds the viewer's controls and keys.
  - `ui/src/features/series-edit/editOptions.ts`: `photoEditOptions(arranged, id)` takes the photos in display order, the series ranges and the pinned ids. `editSeries(edit, gap)` in `series-edit/api.ts`, and the toast texts in `useSeriesMenu.ts` (moved into a shared hook here).
  - `SeriesMenu` (positioned at window pixels, keyboard usable, the WebKit blur fix from testing) can be shown in the viewer as it is.
- How the viewer works today: [Timeline and viewer](../domain/timeline.md#viewer); edits: [Grouping](../domain/grouping.md#series-edited-by-hand), [Editing series](../domain/timeline.md#editing-series).

## Concept

### Only what the strip shows

**The viewer edits only the photos the user sees in it: the current series (the strip) or the current single photo** (decision of the review, 2026-11-19). An edit there must not reach photos outside the strip, which the user cannot see from the viewer. So the viewer offers:

- **„Podziel serię przed tym zdjęciem”** and **„Wyjmij z serii”**, on the shown photo or on any photo of the strip, under the same rules as the grid (a cut only in the middle of a series of three or more, taking out only in a series of three or more);
- **„Rozdziel serię”**, which only splits the series of the strip into its own photos (assumed to count as "splitting"; to confirm in the review).

It does **not** offer:

- „Dołącz do poprzedniej/następnej serii”: it moves the photo into a series outside the strip;
- „Przywróć automatyczne grupowanie”: it gives the whole stretch of edited photos around the photo back to automatic grouping, which reaches photos taken out of the series earlier (now outside the strip) and may merge the series with its neighbours at the current gap.

Both stay in the grid. A single photo therefore has nothing to edit in the viewer.

This also makes the data simple: the viewer's page already holds the strip's photos in display order, which is all `photoEditOptions` needs for these three edits. A pure function builds a small `Arranged` from the page (the strip's photos as entries, the series as one range; for a single photo, the photo alone and no series), runs `photoEditOptions` unchanged and keeps only the viewer's edits (`cut`, `remove`, `ungroup`). So the grid and the viewer still share one set of rules. No neighbours outside the series are needed.

### Candidate approaches: controls

1. **A bar button with a menu (recommended)**: an icon-only „Edytuj serię” button (pencil, tooltip „Edytuj serię (Shift+F10)”) between the photo's info and „Zaakceptuj”, opening the `SeriesMenu` below it with the edits of the shown photo. Visible, so the feature is found; disabled (not hidden, so the bar does not shift) when the photo has no edit to offer, e.g. a single photo or a series of one. Costs one icon button of width in the bar (the narrow-window threshold at 780 px is checked again).
2. **Context menus (recommended, together with 1)**: right-click on the photo opens the menu of the shown photo; right-click on a strip thumbnail opens the menu of *that* photo, without stepping to it, as a right-click on a grid tile does. Shift+F10 / the Menu key open the menu of the focused strip thumbnail, or of the shown photo when nothing else is focused (the viewer's focus is usually on the body). Same habits as in the grid.
3. **Direct keys for the frequent edits**: the Try is a stepping flow ("step, step, cut, step, take out"), where opening a menu each time is slow. One key each for „Podziel serię przed tym zdjęciem” and „Wyjmij z serii” on the shown photo. Proposed keys: **P** (Podziel) and **W** (Wyjmij), free in the keymap and not the X/Space that [slice 60](60-select-and-drag-photos.md) considers for selecting. Both act once per press (`ONCE_PER_PRESS`), since an edit is not part of „Cofnij”. A key whose edit the photo does not offer (P on the first photo of a series) does nothing, like ← on the first photo. „Rozdziel serię” stays in the menu. The menu shows the key next to the two entries.
4. **A strip toolbar** (edit buttons around the strip): always visible, but crowds the strip.

Recommended: 1 + 2 + 3.

### What the viewer shows after an edit

- **The shown photo stays shown.** No edit removes a photo from the timeline, so nothing has to move on; the viewer refetches its page on the timeline-changed event (as after a rejection), so the strip, previous/next and the pencil follow, and so does the grid (its frame stays on the photo).
  - After „Podziel…” the strip shows the photo's new series (it and the photos after it). The photos before it are the previous series now, reached with ← as usual.
  - After „Wyjmij z serii” the photo is a single photo: the strip goes away, and in „Serie” the photo is outside the view, which the viewer already handles (stepping goes to the nearest series photos around it, as for a single photo opened in „Serie”). Moving on to the next photo of the series instead, as after a rejection, would hide the edit's outcome; listed as an open question.
  - An edit from a strip thumbnail's menu keeps the shown photo too; if the shown photo is no longer in the edited photo's series (it was taken out, or the cut put it into the other part), the strip shows the shown photo's series.
  - After „Rozdziel serię” the photo is a single photo, as after „Wyjmij z serii”.
- A toast confirms each edit with the texts of slice 14 („Podzielono serię”, „Wyjęto <name> z serii”, „Rozdzielono serię”); errors as in the grid.
- **Marks:** when the series is edited, the strip starts with the pencil (`EditedIcon`, „Seria edytowana ręcznie – nie zmienia się z odstępem”); a photo pinned as a single photo shows the pinned pencil after its name in the bar („Wyjęte z serii ręcznie – nie łączy się w serię z odstępem”), so the user sees from the viewer what was edited and knows where to restore it (in the grid).
- The viewer has its own toast and its dark look: the menu takes the viewer's dark values through the shared CSS tokens (to check: `SeriesMenu.css` uses `--bg`, `--text-h`, `--series-bg`, …; the viewer sets the shared controls' dark values, possibly not all of these).

## Plan

### Steps

1. **Tauri** (`src-tauri/src/viewer/page.rs`): `Page` gains `series_edited: bool` (the photo's series is `SeriesKind::Manual`) and `pinned: bool` (the photo is in `Grouping::pinned_singles`), for the marks only. Unit tests: an unedited series, an edited series, a pinned single, an unpinned single, an edited series after a gap change.
2. **UI, refactor** (own `refactor:` commit first): `ui/src/features/series-edit/useEditSeries.ts`, holding the toast texts and error mapping now in `useSeriesMenu`, returning `edit(option)`; `useSeriesMenu` uses it. Existing tests stay green.
3. **UI, domain** (`ui/src/features/viewer/viewerEditOptions.ts`, new): `viewerEditOptions(photo: ViewerPhoto, id: number): EditOption[]`, building the strip's `Arranged` and filtering `photoEditOptions` to `cut`, `remove`, `ungroup`. Vitest over the permutations: series of 1, 2, 3, 4 and many; first, middle and last member; the shown photo and another strip photo; a single photo (pinned or not); an edited series; never a join or a restore, also where the grid would offer them.
4. **UI, viewer controls** (`ui/src/features/viewer/`): `EditSeriesButton` in `ViewerBar` (icon-only, `aria-label`/tooltip from `pl.json`), context menus on the photo and the strip thumbnails, a `useViewerSeriesMenu` hook (opens `SeriesMenu` for a photo id at a position, Shift+F10 / Menu key as described, focus back to the opener on Escape). Keymap actions `cutSeries` (P) and `removeFromSeries` (W), once per press, bound with `useShortcuts` in the viewer; the menu shows their keys. Strip pencil and bar badge. `pl.json` strings („Edytuj serię”, „Edytuj serię (Shift+F10)”). Check the narrow-bar threshold with the new button and adjust it.
5. **Tests**: Vitest for the button (disabled without options), the strip's context menu (menu of another photo than the shown one), P/W (including no-op cases and held keys), toasts; Playwright (mocked, `ui/e2e/viewer-series-edit.spec.ts`): cut and take out from the viewer with keys, a cut from a strip thumbnail's menu, „Rozdziel serię”, the strip and the grid following; in both view modes. No real-app spec: nothing here needs the real windows beyond what `follow-viewer` covers.
6. **Testing in the real app** (debug bundle, computer use; every test edit taken back afterwards with „Przywróć automatyczne grupowanie” in the grid, and the action log checked): cut in the middle of a long series, cut again in the new part, take out the first/middle/last photo, take out down to two photos (only „Rozdziel serię” left), cut and take out from a strip thumbnail other than the shown one, ungroup, P on a first photo and W on a single photo (nothing happens), series of 1 and 2 (button disabled or only „Rozdziel serię”), both view modes, a gap change while the viewer is open on an edited series, an edit in the grid while the viewer shows the same series (and the other way round), a narrow viewer window, restart.
7. **Docs**: [Timeline](../domain/timeline.md) (Viewer, Editing series: the viewer's edits, controls and keys, what is shown after an edit, why joins and restoring stay in the grid); this file and the roadmap.

### Open questions for the review

1. Does „Rozdziel serię” belong in the viewer (it only touches the strip's photos), or only cut and take out?
2. Direct keys P and W: yes, other keys, or menu only?
3. After „Wyjmij z serii”, stay on the photo (recommended) or move on to the next photo of the series, as after a rejection?
4. The bar button: pencil icon only (recommended, the bar is full) or with the label „Edytuj”?

## Related slices

- [Slice 60](60-select-and-drag-photos.md) (selection, drag & drop) stays a grid feature; it can reuse `useEditSeries` for its edits and toasts. Keys taken here: P, W.
