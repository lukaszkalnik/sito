---
status: 📝 planned
---

# Slice 59 – Edit series in the viewer

Split off from [slice 14](14-edit-series.md) in its review (2026-11-18). Plan drafted on 2026-11-19, waiting for the user's review.

## Idea

The edits of slice 14 („Podziel serię przed tym zdjęciem”, „Wyjmij z serii”, „Dołącz do poprzedniej/następnej serii”, „Rozdziel serię”, „Przywróć automatyczne grupowanie”) also in the viewer, where the user looks closely at a series: from the bar or the series strip, with keys from `ui/src/shortcuts/keymap.ts`.

## Try

Step through a chained series in the viewer, cut it where the moment changes and take out a stray photo, without going back to the main window; the strip and the grid follow.

## Builds on

- [Slice 14](14-edit-series.md): pinned membership, the `edit_series` command and the pure function listing the possible edits for a photo. This slice only adds the viewer's controls and keys.
  - `ui/src/features/series-edit/editOptions.ts`: `photoEditOptions(arranged, id)` takes the photos in display order, the series ranges and the pinned ids; the viewer can build that from its own page data. `editSeries(edit, gap)` in `series-edit/api.ts`, and the toast texts in `useSeriesMenu.ts` (worth moving into a shared hook when the viewer needs them).
  - `SeriesMenu` (positioned at window pixels, keyboard usable, the WebKit blur fix from testing) can be shown in the viewer as it is.
- How the viewer works today: [Timeline and viewer](../domain/timeline.md#viewer); edits: [Grouping](../domain/grouping.md#series-edited-by-hand), [Editing series](../domain/timeline.md#editing-series).

## Concept

### What the viewer is missing today

`photoEditOptions` needs, around the photo, what the grid has for the whole timeline. The viewer's page (`src-tauri/src/viewer/page.rs`) already has the photo's series (the strip, in display order), but not:

1. **The display neighbours outside the series**: the photo shown just before the series' first member and just after its last one (for a single photo: just before and after it), and whether each of them is in a series. „Dołącz do poprzedniej/następnej serii” depends on them. These are neighbours in the **display order of all photos**, as in the grid (whose `Arranged` always holds all photos, also while „Serie” hides the single ones); they are not the viewer's `previous`/`next`, which follow the view and skip single photos in „Serie”.
2. **Whether the series is edited** (for „Przywróć automatyczne grupowanie” and the pencil).
3. **Whether the photo is pinned as a single photo** (for „Przywróć…” on a photo taken out of a series, and its pencil badge).

### Candidate approaches: getting the data

1. **A local window of the arrangement on the page (recommended).** The page gains `edit: { seriesEdited, pinned, before, after }`, where `before`/`after` are `{ id, inSeries }` of the display neighbours outside the series (`null` at the ends of the timeline). A pure function in the viewer feature builds a small `Arranged` from it: `[before?, ...series members (or the photo alone), after?]`, the photo's series as one range, and each neighbour that is in a series as a range of its own (a different `Series` object, which is all `joinOption` compares). `photoEditOptions` then runs unchanged, for the shown photo and for any photo of the strip. Small payload, no logic duplicated, and the viewer keeps loading one photo at a time.
2. **The viewer loads the whole timeline and series** (`getTimeline` + `getSeries`), as the grid does. Reuses `useTimelineEntries` as it is, but contradicts the viewer's design ([Viewer](../domain/timeline.md#viewer): it never loads the whole timeline) and costs a 50 000-entry load in the second window after every change.
3. **The backend lists the edits** (port `editOptions.ts` to Rust and send the options with the page). One source of truth for the viewer, but then the grid and the viewer use two implementations of the same rules, which drift apart (slice 14's testing already changed these rules three times).

### Candidate approaches: controls

1. **A bar button with a menu (recommended)**: an icon-only „Edytuj serię” button (pencil, tooltip „Edytuj serię (Shift+F10)”) between the photo's info and „Zaakceptuj”, opening the `SeriesMenu` below it with the edits of the shown photo. Visible, so the feature is found; disabled (not hidden, so the bar does not shift) when the photo has no edit to offer. Costs one icon button of width in the bar (the narrow-window threshold at 780 px is checked again).
2. **Context menus (recommended, together with 1)**: right-click on the photo opens the menu of the shown photo; right-click on a strip thumbnail opens the menu of *that* photo, without stepping to it, as a right-click on a grid tile does. Shift+F10 / the Menu key open the menu of the focused strip thumbnail, or of the shown photo when nothing else is focused (the viewer's focus is usually on the body). Same habits as in the grid.
3. **Direct keys for the frequent edits**: the Try is a stepping flow ("step, step, cut, step, take out"), where opening a menu each time is slow. One key each for „Podziel serię przed tym zdjęciem” and „Wyjmij z serii” on the shown photo. Proposed keys: **P** (Podziel) and **W** (Wyjmij), free in the keymap and not the X/Space that [slice 60](60-select-and-drag-photos.md) considers for selecting. Both act once per press (`ONCE_PER_PRESS`), since an edit is not part of „Cofnij”. A key whose edit the photo does not offer (P on the first photo of a series) does nothing, like ← on the first photo. The joins, „Rozdziel serię” and „Przywróć…” stay in the menu: they are rarer and harder to undo by accident. The menu shows the key next to the two entries.
4. **A strip toolbar** (edit buttons around the strip): always visible, but crowds the strip and leaves single photos (no strip) without controls.

Recommended: 1 + 2 + 3.

### What the viewer shows after an edit

- **The shown photo stays shown.** Every edit keeps every photo on the timeline, so nothing has to move on; the viewer refetches its page on the timeline-changed event (as after a rejection), so the strip, previous/next and the pencil follow, and so does the grid (its frame stays on the photo, which may move in the display order, e.g. a photo joining a series of another camera).
  - After „Podziel…” the strip shows the photo's new series (it and the photos after it).
  - After „Wyjmij z serii” the photo is a single photo: the strip goes away, and in „Serie” the photo is outside the view, which the viewer already handles (stepping goes to the nearest series photos around it, as for a single photo opened in „Serie”). The alternative, moving on to the next photo of the series as after a rejection, makes the edit's outcome invisible and is not what the user asked for; it is listed as an open question.
- A toast confirms each edit with the texts of slice 14 („Podzielono serię”, „Wyjęto <name> z serii”, …); errors as in the grid.
- **Marks:** when the series is edited, the strip starts with the pencil (`EditedIcon`, „Seria edytowana ręcznie – nie zmienia się z odstępem”); a photo pinned as a single photo shows the pinned pencil after its name in the bar („Wyjęte z serii ręcznie – nie łączy się w serię z odstępem”).
- The viewer has its own toast and its dark look: the menu takes the viewer's dark values through the shared CSS tokens (to check: `SeriesMenu.css` uses `--bg`, `--text-h`, `--series-bg`, …; the viewer sets the shared controls' dark values, possibly not all of these).

## Plan

### Steps

1. **Tauri** (`src-tauri/src/viewer/page.rs`, possibly a submodule `viewer/page/edit.rs` if the file grows past ~400 lines): `Page` gains `edit: EditContextDto { series_edited, pinned, before: Option<NeighbourDto>, after: Option<NeighbourDto> }`, `NeighbourDto { id, in_series }`. `series_edited` from the kind of the photo's series (`SeriesKind::Manual`), `pinned` from `Grouping::pinned_singles`, the neighbours from the arrangement (display positions `series.start - 1` and `series.start + series.len`, or `shown ± 1` for a single photo). Unit tests: neighbours at both ends of the timeline, single/series neighbours on each side, a single photo between two series, an edited series, a pinned single, an unedited series after a gap change, interleaved cameras (neighbour of another camera inside the time span).
2. **UI, domain** (`ui/src/features/viewer/viewerArranged.ts`, new): `viewerArranged(photo: ViewerPhoto): Arranged`, the local window of approach 1. Vitest over the permutations: series of 1, 2, 3 and many; first, middle and last member; single photo with no neighbours, single/series neighbours on each side; edited and unedited series; pinned single. Checks through `photoEditOptions` that the viewer gets the same options as the grid for the same situation (a shared fixture building the grid's full `Arranged` and the viewer's window from one timeline).
3. **UI, shared hook** (`ui/src/features/series-edit/useEditSeries.ts`, refactor in its own `refactor:` commit first): the toast texts and error mapping move out of `useSeriesMenu` into `useEditSeries(gap, showToast)` returning `edit(option)`; `useSeriesMenu` uses it. Existing tests stay green.
4. **UI, viewer controls** (`ui/src/features/viewer/`): `EditSeriesButton` in `ViewerBar` (icon-only, `aria-label`/tooltip from `pl.json`), context menus on the photo and the strip thumbnails, a `useViewerSeriesMenu` hook (opens `SeriesMenu` for a photo id at a position, Shift+F10 / Menu key as described, focus back to the opener on Escape). Keymap actions `cutSeries` (P) and `removeFromSeries` (W), once per press, bound with `useShortcuts` in the viewer; the menu shows their keys. Strip pencil and bar badge. `pl.json` strings („Edytuj serię”, „Edytuj serię (Shift+F10)”). Check the narrow-bar threshold with the new button and adjust it.
5. **Tests**: Vitest for the button (disabled without options), the strip's context menu (menu of another photo than the shown one), P/W (including no-op cases and held keys), toasts; Playwright (mocked, `ui/e2e/viewer-series-edit.spec.ts` or extending `series-edit.spec.ts`): cut and take out from the viewer with keys, a join from the strip menu, „Przywróć…” on a pinned photo, the strip and the grid following; in both view modes. No real-app spec: nothing here needs the real windows beyond what `follow-viewer` covers.
6. **Testing in the real app** (debug bundle, computer use; every test edit taken back with „Przywróć automatyczne grupowanie” and the action log checked): cut in the middle of a long series, cut again in the new part, take out the first/middle/last photo, join a single photo to the series before and after it (by strip menu and by photo menu), ungroup and restore from the viewer, P on a first photo and W on a single photo (nothing happens), both view modes, a gap change while the viewer is open on an edited series, an edit in the grid while the viewer shows the same series (and the other way round), a narrow viewer window, restart.
7. **Docs**: [Timeline](../domain/timeline.md) (Viewer, Editing series: the viewer's controls and keys, what is shown after an edit), [Grouping](../domain/grouping.md#series-edited-by-hand) only if behaviour changes (it should not); this file and the roadmap.

### Open questions for the review

1. Direct keys P and W (approach 3): yes, other keys, or menu only?
2. After „Wyjmij z serii”, stay on the photo (recommended) or move on to the next photo of the series, as after a rejection?
3. The bar button: pencil icon only (recommended, the bar is full) or with the label „Edytuj”?
4. Should P and W also work on a focused tile in the grid? Cheap with the shared hook, but outside this slice's idea; could be a small follow-up.

## Related slices

- [Slice 60](60-select-and-drag-photos.md) (selection, drag & drop) stays a grid feature; it can reuse `useEditSeries` for its edits and toasts. Keys taken here: P, W.
