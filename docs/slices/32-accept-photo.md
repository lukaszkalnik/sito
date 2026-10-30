# Slice 32 – Accept a photo

**Status:** ✅ done (tested by the user on 2026-10-30; what it established is in [docs/domain/accepting.md](../domain/accepting.md))

## Context

Split out of [slice 12](12-user-feedback-1.md) (item 3) on 2026-10-29: it needs its own storage and undo, unlike the quick viewer fixes there. From the user's first real test (2026-10-27).

## Approved plan (2026-10-30)

From the user's first real test: in a big series the user wants to mark the keeper(s) before rejecting the rest. „Zaakceptuj” marks a photo as accepted; a green check shows on its thumbnails (grid and series strip), it survives restarts and rescans, and it is undoable like a rejection.

Decisions (2026-10-30, user):
- Several photos of one series may be accepted.
- After accepting, the viewer stays on the photo.
- Key **A**.
- Toggle: on an accepted photo the button reads „Cofnij akceptację” and A takes the mark off; „Cofnij”/Ctrl+Z undoes the newest action, accept, un-accept or reject alike.

## Behaviour

- Viewer bar: „Zaakceptuj” (green, check icon) left of „Odrzuć”. On an accepted photo it is pressed (`aria-pressed`) and reads „Cofnij akceptację”. Hint shows the key (A). Toast: „Zaakceptowano <name>” / „Cofnięto akceptację <name>”, both with „Cofnij”.
- Green check badge on accepted photos: grid tiles and series-strip items only, not on the big photo. Badge has `aria-label`/title „Zaakceptowane”.
- „Cofnij” is one stack, newest first, per open folder: undoing an accept removes the mark, undoing an un-accept puts it back; toasts „Cofnięto akceptację <name>” / „Przywrócono akceptację <name>”.
- Acceptance is keyed by the photo's path. A rejected accepted photo keeps its row; undoing the rejection brings it back accepted. A photo moved or renamed outside Sito loses its mark (accepted).
- Hold A does not repeat (once per press, like Delete).

## Core (`crates/core`)

- `actions.rs`: migration 4 `CREATE TABLE accepted (path TEXT PRIMARY KEY)`. New action kinds `accept` / `unaccept` (from_path = to_path = photo path).
- Generalize undo lookup: `last_undoable(under)` returns `Option<LoggedAction>`:
  `enum LoggedAction { Reject(RejectAction), Accept(AcceptAction) }`, `AcceptAction { id, path, change: AcceptChange }`, `enum AcceptChange { Accepted, Unaccepted }`. `last_row_under` includes the two new kinds. Existing `Placement` mapping moves into a helper so `last_undoable` stays short.
- New `crates/core/src/accept.rs` (data layer, `impl ActionLog` block; `conn` becomes `pub(crate)`), keeps `actions.rs` under the size limit:
  - `set_accepted(path, AcceptChange) -> Result<Option<i64>, LogError>`: in **one transaction** insert/delete the `accepted` row and write the log entry. A no-op (already in that state, e.g. a second window) returns `Ok(None)` and logs nothing.
  - `undo_accept(&AcceptAction)`: one transaction reverting the row and setting `undone_at`.
  - `accepted_paths(under) -> HashSet<PathBuf>`.
- Tests (in-memory log): accept/unaccept/undo round trips, no-op not logged, undo order mixes rejects and accepts, `under` filtering, survives reopen (file DB in temp dir), migration from a v3 DB keeps old entries.

## Tauri (`src-tauri`)

- New `src-tauri/src/accept.rs`: commands `accept_photo(id)`, `unaccept_photo(id)` (two commands, no bool flag), `get_accepted_ids() -> Vec<usize>` (ids of current scan photos whose path is accepted; set copied from DB, scan locked only to map). Emits new event `ACCEPTED_CHANGED` (`accepted-changed`). Returns `AcceptedDto { name }`. Uses the `Actions` state from `reject.rs` (make its log accessor `pub(crate)`).
- `reject.rs` undo: rename command `undo_reject` → `undo_last` (refactor commit first). `undo_in` matches `LoggedAction`: Reject as today (emits `TIMELINE_CHANGED`), Accept → `log.undo_accept`, emits `ACCEPTED_CHANGED`. `UndoneDto` gains `kind: UndoneKind { Restored, Unaccepted, Reaccepted }`. `get_undo_state` unchanged in shape (uses `last_undoable`).
- Register commands in `lib.rs`. Tests next to the existing ones in `reject.rs` (gallery fixtures): accept then reject then undo twice; get_accepted_ids after restart.

## UI (`ui/src/features/accept/`)

- `api.ts`: `acceptPhoto`, `unacceptPhoto`, `getAcceptedIds`, `onAcceptedChanged` (typed wrappers).
- `useAcceptedIds.ts`: `ReadonlySet<number>` for the current scan, reloaded on `accepted-changed`, timeline revision and scan change. Used by the grid and the viewer.
- `AcceptedBadge.tsx` (+css): green check SVG with translated `aria-label`.
- `AcceptButton.tsx`: toggling button for the viewer bar.
- `reject/api.ts` + `useUndo.ts`: `undoLast`, `Undone.kind`; toast text per kind; `useUndoAvailable` also listens to `accepted-changed`.
- Timeline: `TimelineRowView` passes `accepted` to `TimelineTile`, which renders `AcceptedBadge`. Viewer: `PhotoViewer` gets `accepted`/`onToggleAccept`; `SeriesStrip` shows the badge on accepted members; `useViewerKeys` binds `accept`.
- `shortcuts/keymap.ts`: action `accept` → `{ key: 'a', modifier: 'none' }`, added to `ONCE_PER_PRESS`.
- `pl.json`: `accept.button` „Zaakceptuj”, `accept.undoButton` „Cofnij akceptację”, `accept.hint` „Oznacz zdjęcie jako zaakceptowane (A)”, `accept.done`, `accept.undone`, `accept.badge` „Zaakceptowane”, `accept.failed`; `undo.unaccepted`, `undo.reaccepted`; `undo.hint` → „Cofnij ostatnią czynność (Ctrl+Z)”.
- Tests: Vitest for `useAcceptedIds`, the badge on a tile, the viewer button toggle + key A, undo toast per kind; keymap uniqueness test already covers the new key. Playwright: extend the mocked-Tauri e2e with accept → badge in grid → Cofnij removes it.

## Docs

- New `docs/domain/accepting.md` (behaviour above), indexed in `docs/domain/README.md`; `rejecting.md` and `timeline.md` link to the shared undo and the new button/badge.
- Slice file `docs/slices/32-accept-photo.md`: decisions + this plan, status 🚧 → 🧪; roadmap in sync. Note in slice 16 that accepted photos could become a view using `accepted_paths`.

## Commits (each with checks green, pushed to `main`)

1. `docs:` save the plan into slice 32.
2. `refactor:` rename `undo_reject` → `undo_last` (no behaviour change).
3. `feat(core):` accepted table, accept/unaccept log, generalized undo lookup + tests.
4. `feat:` Tauri commands/event + UI button, key, badges, undo toasts + tests, e2e.
5. `docs:` domain files, slice status 🧪, roadmap.

## Verification

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e` (one at a time, low RAM).
- Debug build (`npm run tauri dev`) for the user: accept the best shot of a big series → check in grid and strip; A toggles; Ctrl+Z undoes accept, un-accept and rejects in order; restart keeps the check; reject an accepted photo and undo → still accepted.

## Related slices

- [Slice 16](16-show-rejected.md) (rejected photos as a view) could show accepted photos the same way, from `accepted_paths`.

## Implementation notes (2026-10-30)

Done as planned, with these differences:

- The core change and the Tauri commands went into one commit: `last_undoable` returning `LoggedAction` does not compile without the Tauri undo adapting to it.
- `useAcceptedIds` lives in the viewer's `LoadedViewer` (not per photo), so stepping between photos does not reload the ids or flicker the button. Strip items of accepted photos are labelled „<name> (zaakceptowane)”, since a badge inside a button is not part of its name.
- `undo.failed` now reads „Nie udało się cofnąć ostatniej czynności.”, as undo is no longer only about rejections.
- Two small refactors on the way: `SeriesStrip` moved to its own file; the e2e gallery mock (`ui/e2e/gallery.ts`) is shared by `reject.spec.ts` and the new `accept.spec.ts`.

Behaviour is described in [docs/domain/accepting.md](../domain/accepting.md).

### To test (debug build)

Accept the best shot of a big series → check in grid and strip; A toggles; Ctrl+Z undoes accept, un-accept and rejects in order; restart keeps the check; reject an accepted photo and undo → still accepted.
