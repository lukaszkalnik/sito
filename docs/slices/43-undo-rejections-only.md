# Slice 43 – Undo only rejections

**Status:** 📝 concept drafted (2026-11-01; plan in plan mode when the slice starts)

## Idea

Asked by the user on 2026-11-01: „Cofnij” (and Ctrl+Z) should not undo accepting or un-accepting a photo. The „Zaakceptuj” / „Cofnij akceptację” button (and the key A) is enough to take a mark back, so undo is only for rejections, which move files.

Today ([Accepting](../domain/accepting.md#undo), [slice 32](32-accept-photo.md)) „Cofnij” is one stack per open folder for rejections, accepts and un-accepts alike. Accepting in a series and then pressing Ctrl+Z to bring back the last rejected photo first takes the marks off, one by one, which is not what the user expects.

## Candidate behaviour

- „Cofnij” / Ctrl+Z undoes the newest **rejection** not yet undone in the open folder, skipping accepts and un-accepts in between; they stay as they are.
- Accepting and un-accepting still show a confirmation toast („Zaakceptowano <name>”, „Cofnięto akceptację <name>”), but without „Cofnij”.
- „Cofnij” is enabled only while a rejection can be undone; accepting alone no longer enables it.
- The hint goes back to rejections: e.g. „Cofnij ostatnie odrzucenie (Ctrl+Z)”; `undo.failed` likewise.
- A rejected accepted photo still comes back accepted on undo (unchanged).

## Candidate approaches

1. **Keep logging accepts, leave them out of undo (preferred).** Accepts and un-accepts stay in the action log (every action is logged), but the undo lookup (`last_row_under` / `last_undoable` in `crates/core/src/actions.rs`) only considers `reject` and `trash` rows. `LoggedAction::Accept`, `undo_accept` and the `Unaccepted` / `Reaccepted` undo kinds go away. No migration: existing accept rows simply are no longer offered for undo.
2. **Stop logging accepts.** The `accepted` table alone holds the marks. Less code, but breaks the rule that every action is logged; ruled out unless the user prefers it.

## To check when planning

- `src-tauri/src/reject.rs`: `undo_last` / `UndoneDto` / `UndoneKind` shrink back to rejections; whether to rename `undo_last` back (separate `refactor:` commit).
- UI: `ui/src/features/accept/useToggleAccept.ts` toast kind (no „Cofnij”), `ui/src/features/reject/useUndo.ts` toast per kind, `useUndoAvailable` no longer listens to `accepted-changed`; `pl.json` drops `undo.unaccepted` / `undo.reaccepted`.
- Tests: core undo order skips accepts (accept, reject, accept → undo restores the rejection and leaves both marks); Vitest for the accept toast without „Cofnij”; the Playwright accept flow no longer undoes via „Cofnij”.
- Docs: `docs/domain/accepting.md` (Undo section), `rejecting.md` („one stack shared with accepting”), `timeline.md` where it mentions undo.
