---
status: 📝 planned
---

# Slice 14 – Edit series

Plan drafted on 2026-11-18, waiting for the user's review.

## Idea

Fix false positives and misses by hand: remove a photo from a series, add a photo to a neighbouring series, ungroup a series completely, or "cut here" to split a series that chains several moments (e.g. a long event shot in quick succession) into two at a chosen photo. Edits are stored in SQLite (slice 9), survive rescans and win over automatic grouping.

## Try

Cut a chained series into its real moments, ungroup a wrong series and remove a stray photo, rescan, and check that the edits are kept.

## Builds on

- [Slice 33](33-split-by-camera.md): a series is a list of its photos (`Series::members`, timeline positions) with a display order (`Arrangement`) built from it, so series need not be contiguous in time. Edits and splits only need to produce member lists; the grid and viewer follow the arrangement.
- How grouping works today: [Grouping](../domain/grouping.md). Accepted marks ([Accepting](../domain/accepting.md)) are the model for storing something per photo by path in the action log and moving it along with moved photos.

## Candidate approaches

How an edit is stored decides how it behaves when the gap changes, photos are added or trashed, and the folder is rescanned.

1. **Links between photos.** Store forbidden links (a cut between two neighbouring photos of one camera) and forced links (a photo joined to another); series are the connected photos of the automatic links minus the forbidden plus the forced ones. Edits keep working with another gap (a camera's neighbours do not depend on the gap) and a larger gap can still merge an edited series with its neighbours. But a cut is lost when a photo between its two ends appears or one end is trashed, and what an edit means at another gap is hard to explain.
2. **Pinned membership (recommended).** An edit pins the photos it touches: each pinned photo belongs to a manual series or is pinned as a single photo. Pinned photos are taken out of automatic grouping; automatic grouping runs on the rest as today. **An edited series stays exactly as the user left it, whatever the gap.** Easy to explain („edytowana seria nie zmienia się z odstępem”), easy to undo (unpin), and robust: a trashed or vanished member just leaves its series, a new photo is grouped automatically.

The plan below uses approach 2.

## Plan

### Behaviour

- **Edits in the grid**, from a context menu on a tile (right-click, or the Menu key / Shift+F10 on a focused tile) and on a series header:
  - **„Podziel serię przed tym zdjęciem”** (on a series photo other than its first): the photos before it (newer, in display order) stay one series, it and the photos after it form another. A part left with one photo becomes a single photo.
  - **„Wyjmij z serii”** (on a series photo): the photo becomes a single photo at its place in time; the rest stays one series (or a single photo if only one is left).
  - **„Dołącz do poprzedniej serii”** / **„Dołącz do następnej serii”** (on a photo whose previous / next photo in display order belongs to another series): the photo joins that series. If it came from another series, that one keeps the rest.
  - **„Rozdziel serię”** (header): every photo of the series becomes a single photo.
  - **„Przywróć automatyczny podział”** (header of an edited series, and the menu of a pinned single photo): unpins those photos, so they are grouped automatically again.
- A **toast** confirms each edit („Podzielono serię”, „Wyjęto <name> z serii”, „Dołączono <name> do serii”, „Rozdzielono serię”, „Przywrócono automatyczny podział”). Edits are not part of „Cofnij” (it stays for rejections, [slice 43](43-undo-rejections-only.md)); „Przywróć automatyczny podział” takes an edit back.
- **Edited series are marked**: their header gets a small pencil icon after the text, with the tooltip and accessible name „Seria edytowana ręcznie – nie zmienia się z odstępem”.
- **Edits win and last**: an edited series and pinned single photos stay as they are with any gap, after a refresh, a reopened folder and a restart. Rejected photos stay in their manual series (they still count, as for automatic series); photos trashed or gone leave it. A manual series with one photo left shows as a series of one, as automatic series do after rejections; with none left it is gone.
- Moved and renamed photos ([Photos](../domain/photos.md#moved-and-renamed-photos)) keep their pin, like accepted marks.
- Both windows follow: an edit changes the timeline's grouping, so the grid and the viewer (its strip, previous/next) update as after a rejection. The summary's series count follows.
- Labels (camera names on overlapping series) are worked out over manual and automatic series together.

### Steps

1. **Core, domain** (`crates/core/src/series/pins.rs`, new): `Pins` (pinned photos by path: `Pin::Series(ManualSeriesId)` or `Pin::Single`), and `Grouping::new` taking them: automatic grouping on the unpinned photos, manual series from the pins, both sorted by newest photo, labels over all of them. Pure functions that turn an edit (`Edit::Cut { at }`, `Remove`, `Join { photo, into }`, `Ungroup`, `Restore`) on the current grouping into the pin changes (pins to set, pins to clear). Unit tests for every edit, gap changes, rejected and missing members, interleaved cameras.
2. **Core, data** (`crates/core/src/series/pin_store.rs`): migration adding `series_pins (path TEXT PRIMARY KEY, series INTEGER NULL)` (NULL = single) to the action log; `pins_under(folder)`, `apply(changes)` in one transaction, and `move_pins(moves)` next to `move_accepted`. Tests on an in-memory log.
3. **Tauri** (`src-tauri/src/series.rs`, submodule `series/edit.rs`): the scan holds the folder's pins (loaded with the accepted marks); `edit_series(edit)` command with ids of the current scan, applies the edit, stores it, bumps the timeline revision and sends the timeline-changed event; moves from rescans also move pins. `SeriesDto` gains `edited: bool`. Error codes for a stale scan or photo.
4. **UI** (`ui/src/features/series-edit/`, new feature): typed `editSeries` wrapper, a pure function listing the menu entries for a tile or header (tested), the context menu component (keyboard usable, Escape closes, focus returns to the tile), the header pencil, toasts, `pl.json` strings. Menu key added to the keymap if it needs an action there.
5. **Tests**: Vitest for the menu entries and component; Playwright (mocked) for cut, remove, join, ungroup and restore in the grid; a core integration test (`crates/core/tests/`) that edits, rescans and finds the edits kept; one real-app spec only if the restart case needs it (it likely does not: the core test covers persistence).
6. **Docs**: grouping domain file (edits, pins, how they interact with the gap), timeline file (context menu, pencil); this slice's status.

## Questions for the review

1. Approach 2 (pinned membership: an edited series no longer follows the gap) rather than approach 1?
2. Grid only in this slice, or also in the viewer (e.g. „Podziel tutaj” and „Wyjmij z serii” in the strip or bar, with keys)? The viewer is where the user looks closely at a series, but adding it roughly doubles the UI work; it could be a follow-up slice.
3. Is a per-series „Przywróć automatyczny podział” enough, or should there also be one for the whole folder?

## Later slices

- [Slice 19](19-split-by-content.md) (split by content) should leave pinned photos alone: it splits automatic series only, so it can run on the unpinned photos like the time grouping does.
