---
status: 📝 concept drafted
---

# Slice 43 – Undo only rejections

Since 2026-11-01; plan in plan mode when the slice starts.

## Idea

Asked by the user on 2026-11-01: „Cofnij” (and Ctrl+Z) should not undo accepting or un-accepting a photo. The „Zaakceptuj” / „Cofnij akceptację” button (and the key A) is enough to take a mark back, so undo is only for rejections, which move files.

Today ([Accepting](../domain/accepting.md#undo), [slice 32](32-accept-photo.md)) „Cofnij” is one stack per open folder for rejections, accepts and un-accepts alike. Accepting in a series and then pressing Ctrl+Z to bring back the last rejected photo first takes the marks off, one by one, which is not what the user expects.

## Candidate behaviour

- „Cofnij” / Ctrl+Z undoes the newest **rejection** not yet undone in the open folder, skipping accepts and un-accepts in between; they stay as they are.
- Accepting and un-accepting still show a confirmation toast („Zaakceptowano <name>”, „Cofnięto akceptację <name>”), but without „Cofnij”.
- „Cofnij” is enabled only while a rejection can be undone; accepting alone no longer enables it.
- The hint goes back to rejections: e.g. „Cofnij ostatnie odrzucenie (Ctrl+Z)”; `undo.failed` likewise.
- A rejected accepted photo still comes back accepted on undo (unchanged).

## Approach

Decided by the user on 2026-11-01: **accepts and un-accepts are no longer logged.** The `accepted` table alone holds the marks; the action log is for file moves (rejections), which are what undo is for. The "every action is logged" rule in `CLAUDE.md` is about rejected photos, so it still holds.

- `crates/core/src/actions.rs`: accepting and un-accepting only change the `accepted` table (no `accept` / `unaccept` rows). The undo lookup (`last_row_under` / `last_undoable`) only considers `reject` and `trash` rows. `LoggedAction::Accept`, `AcceptAction`, `AcceptChange` and `undo_accept` go away.
- Existing `accept` / `unaccept` rows in users' logs stay untouched (durable data, no migration edits); they are simply never offered for undo again. Whether to keep the `KIND_ACCEPT` / `KIND_UNACCEPT` constants only for documenting old rows is decided when planning.

## To check when planning

- `src-tauri/src/reject.rs`: `undo_last` / `UndoneDto` / `UndoneKind` shrink back to rejections; whether to rename `undo_last` back (separate `refactor:` commit).
- UI: `ui/src/features/accept/useToggleAccept.ts` toast kind (no „Cofnij”), `ui/src/features/reject/useUndo.ts` toast per kind, `useUndoAvailable` no longer listens to `accepted-changed`; `pl.json` drops `undo.unaccepted` / `undo.reaccepted`.
- Tests: accepting writes no log row; undo order ignores accepts (accept, reject, accept → undo restores the rejection and leaves both marks); old `accept` rows in a log are not offered for undo; Vitest for the accept toast without „Cofnij”; the Playwright accept flow no longer undoes via „Cofnij”.
- Docs: `docs/domain/accepting.md` (Undo section), `rejecting.md` („one stack shared with accepting”), `timeline.md` where it mentions undo.
