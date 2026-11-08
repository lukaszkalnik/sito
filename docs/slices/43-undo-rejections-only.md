---
status: ✅ done
---

# Slice 43 – Undo only rejections

Concept since 2026-11-01; plan approved on 2026-11-08 (below); implemented on 2026-11-08 (commits 2–5 of the plan); tested by the user and done on 2026-11-08 without fixes. Its knowledge is in [Accepting](../domain/accepting.md#undo) and [Rejecting](../domain/rejecting.md).

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

- `crates/core/src/actions.rs`: accepting and un-accepting only change the `accepted` table (no `accept` / `unaccept` rows). The undo lookup (`last_row_under` / `last_undoable`) only considers `reject` and `trash` rows. `LoggedAction::Accept`, `AcceptAction`, `AcceptChange` and `undo_accept` go away (the plan below keeps `AcceptChange` as the input of `set_accepted`).
- Existing `accept` / `unaccept` rows in users' logs stay untouched (durable data, no migration edits); they are simply never offered for undo again. Whether to keep the `KIND_ACCEPT` / `KIND_UNACCEPT` constants only for documenting old rows is decided when planning.

## Plan (approved 2026-11-08)

### Context

Today „Cofnij” / Ctrl+Z is one stack per folder for rejections, accepts and un-accepts ([slice 32](32-accept-photo.md)). If you accept a photo and then press Ctrl+Z to bring back the last rejected photo, it first takes the marks off one by one. The user decided on 2026-11-01 that accepting and un-accepting are **no longer logged**. The `accepted` table alone holds the marks, „Zaakceptuj” / „Cofnij akceptację” / A takes a mark back, and undo is only for rejections (file moves). Expected result: Ctrl+Z always restores the newest rejection that is not undone yet, and accepts in between stay as they are.

### Decisions

- **No migration.** Old `accept` / `unaccept` rows stay in users' logs, untouched. The undo query filters on `kind IN ('reject', 'trash')`, so they are never offered again. `KIND_ACCEPT` / `KIND_UNACCEPT` go away. A comment at migration 4 in `MIGRATIONS` notes that logs written by v0.x (slices 32–42) may hold such rows and that undo ignores them.
- **`AcceptChange` stays** as the input of `set_accepted` (Accepted / Unaccepted, an enum, not a bool flag). Its `reverse()` and `kind()` go away. `AcceptAction`, `LoggedAction::Accept`, `undo_accept` go away.
- **`LoggedAction` goes away:** `last_undoable` returns `Option<RejectAction>`. If slice 16 adds an undoable "restore" kind, it can bring back an enum then. A short note about this goes into slice 16's file.
- **`UndoneKind` goes away; `UndoneDto` = `{ id, name }`.** Undo always emits `timeline-changed`. UI `Undone` loses `kind`, and the toast is always `undo.done`.
- **Rename `undo_last` → `undo_reject`** (its name before slice 32) and `undoLast` → `undoReject`, as a separate `refactor:` commit after the feature commit.
- **Strings:** `undo.hint` → „Przywróć ostatnio odrzucone zdjęcie (Ctrl+Z)” and `undo.failed` → „Nie udało się cofnąć odrzucenia.” (both the wording used before slice 32). `undo.unaccepted` / `undo.reaccepted` are dropped. `accept.done` / `accept.undone` stay.
- `set_accepted` returns `Result<(), LogError>` (the id is no longer needed; the "already in that state" case simply changes nothing).

### Changes

### core (`crates/core`)
- `src/accept.rs`: `set_accepted` only calls `write_mark` in its transaction, without inserting into `actions`. Remove `AcceptAction`, `undo_accept`, `AcceptChange::reverse/kind`. Update the module doc (no longer "every change can be undone").
- `src/actions.rs`: module doc → the log of file moves. Remove `LoggedAction` and the accept kinds. `last_row_under` queries `kind IN (?1, ?2)` with reject/trash. `last_undoable` → `Option<RejectAction>` via `reject_action`. Add the migration 4 comment.
- `tests/accept.rs`: rewrite:
  - accepting and un-accepting write no `actions` row (`last_undoable` is `None`; count rows through a raw `rusqlite` connection on a file DB).
  - accept, reject, accept → `last_undoable` is the rejection; after `mark_undone` it is `None`; both marks stay.
  - an old `accept` / `unaccept` row inserted with raw SQL into a file DB is not offered (`None`), and a reject before it still is.
  - keep the tests for the sibling folder, marks across reopening and upgrading a v3 log (adjusted to the new types).
- `tests/actions.rs`: drop the `LoggedAction` match helper (it now returns `RejectAction` directly).

### src-tauri
- `src/reject.rs`: drop `UndoneKind`, `kind` from `UndoneDto`, the `LoggedAction` match and the `accept::undo` call. `undo_in` → `undo_reject(...)` directly, and announce `TIMELINE_CHANGED`. Module doc: "undo the newest rejection". Tests: replace `undo_takes_back_a_rejection_and_then_the_accept_before_it` and `undoing_an_unaccept_accepts_the_photo_again` with "accept, reject, accept → undo restores the photo, both marks stay, next undo is `None`". Update the DTO serialization test (`{ id, name }`).
- `src/accept.rs`: remove `undo` and the `UndoneDto/UndoneKind` imports. Command docs: "and logs it" → no logging. `set_mark` returns `()` from core.
- Refactor commit: `undo_last` → `undo_reject` (`lib.rs` registration, `reject.rs`, `ui/src/features/reject/api.ts`, `ui/e2e/tauri-mock.ts`).

### UI (`ui/`)
- `features/reject/api.ts`: remove `UndoneKind` and `kind` from `Undone`; update the docs.
- `features/reject/useUndo.ts`: remove `UNDONE_KEYS`, always use `t('undo.done', …)`. `useUndoAvailable` listens only to `onTimelineChanged`. Update the docs.
- `features/accept/useToggleAccept.ts`: toasts use `kind: 'info'` (no „Cofnij”).
- `i18n/pl.json`: string changes as above.
- Vitest: `useToggleAccept.test.ts` expects `kind: 'info'`. `useUndo.test.tsx` drops the kind cases and checks that `accepted-changed` no longer refreshes availability (if it is mocked there). Check `PhotoViewer.test.tsx` / `TimelineView.test.tsx` / `ScanScreen.test.tsx` for `kind:` in `Undone` fixtures and the old hint text.
- e2e: in `tauri-mock.ts`, accept/unaccept no longer push to `undoable`, and `undoAction` handles only rejects. In `accept.spec.ts`:
  - "undoes accepting and rejecting in one stack" → "undo skips accepts": accept, reject, accept the next photo, Ctrl+Z → the rejected photo is back and accepted, and „Cofnij” is disabled.
  - "takes the mark off with Cofnij" → accepting shows the toast without a „Cofnij” button, and „Cofnij” stays disabled.
  - Add a check that after accepting alone, „Cofnij” is disabled.

### Docs
- `docs/domain/accepting.md`: the toast has no „Cofnij”. Replace the Undo section: undo is for rejections only and does not touch marks; old logged accepts are ignored. Keep "a rejected accepted photo comes back accepted".
- `docs/domain/rejecting.md`: drop "one stack shared with accepting photos".
- `docs/domain/README.md`: Accepting row without "and the undo stack shared with rejecting".
- `docs/domain/timeline.md`: no change needed (checked: it has no accept-undo mentions).
- [slice 16](16-show-rejected.md): the note above. Slice 43 file: approved plan, then status updates. Roadmap in sync.

### Amendment (2026-11-08): rejections into the macOS trash

Sito cannot put a photo back from the macOS trash yet (the `trash` crate only restores on Windows and Linux; a way to do it is [slice 23](23-macos-polish.md)). Until then, decided by the user on 2026-11-08:

- On macOS, „Cofnij” / Ctrl+Z **skips trash moves** and undoes the newest rejection into „Odrzucone”. The skipped entries stay in the log and are not marked undone, so they become undoable once slice 23 can restore them.
- „Cofnij” **looks inactive only if every rejection still to undo went to the trash**. Clicking it or pressing Ctrl+Z then shows an error toast „Na tym systemie nie można cofnąć przeniesienia do kosza – przywróć zdjęcie z Kosza ręcznie.” (`undo.trashUndoUnsupported`, reworded from „…z Kosza.”); its tooltip stays `undo.hint`. A toast instead of a tooltip (user, 2026-11-08), since WebKit shows no tooltip on a `disabled` button. With no rejection at all it is `disabled` with the usual hint, as today.
- Windows and Linux are unchanged.

Changes on top of the plan above:

- **core:** `last_undoable(under, rejections)` with `enum Rejections { All, FolderOnly }` (`FolderOnly` queries `kind = 'reject'`). `crates/core/src/trash.rs` gets a `TRASH_RESTORE: TrashRestore` const (`Supported` / `Unsupported`, set by `cfg`) next to `restore_from_trash`. Tests: with a folder and a newer trash rejection, `FolderOnly` returns the folder one and `All` the trash one; neither is marked undone by the lookup.
- **src-tauri:** the `Bin` trait reports whether it can restore (the system bin from `TRASH_RESTORE`, `fake_bin` configurable), and `undo_in` and `get_undo_state` pick `Rejections` from it. `UndoStateDto` becomes a tagged enum `{ kind: "available" } | { kind: "none" } | { kind: "trashOnly" }`. `trashOnly` is when `FolderOnly` finds nothing but `All` does. `TrashUndoUnsupported` stays as the error code for a restore that is refused anyway. Tests with a non-restoring fake bin: folder, trash → undo restores the folder one; then the state is `trashOnly`; serialization of the three states.
- **UI:** `api.ts` `undoState()` returns that union (validated, unknown → `none`) instead of `canUndo()`. `useUndo` exposes it; for `trashOnly`, its `undo` (button and Ctrl+Z) does not call the backend and shows the error toast instead. The „Cofnij” buttons in `TimelineView.tsx`, `PhotoViewer.tsx` and `ViewerApp.tsx` (`EmptyViewer`) take the state: enabled for `available`; for `trashOnly`, `aria-disabled="true"` (styled like a disabled button, still focusable and clickable for the toast); for `none`, `disabled`. Since the three buttons repeat this, extract one `UndoButton` in `features/reject/` (separate `refactor:` commit first). Vitest for the three states (query by role and name, check `aria-disabled` and the toast). The e2e mock gains an option for a non-restoring trash; one Playwright test: rejected into the trash only → „Cofnij” looks inactive, and clicking it shows the toast.
- **Docs:** `docs/domain/rejecting.md` (the macOS bullet), and slice 23's file notes that its restore turns the skipped trash entries back on.

### Commits (on `main`, pushed after each)
1. `docs: save the approved plan of slice 43` (slice file + slice 16 note). Then **pause** (milestone).
2. `feat: undo only rejections, stop logging accepts` (core + tauri + UI + tests + domain docs).
3. `refactor: share one UndoButton between the windows`.
4. `feat: skip rejections into the macOS trash on undo` (the amendment).
5. `refactor: rename undo_last back to undo_reject`.
6. `docs: mark slice 43 ready for testing` (🧪, roadmap). Then pause.

### Verification
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e` (accept + reject specs)
- Manual in `npm run tauri dev`: reject a photo, accept two others, Ctrl+Z → the rejected photo is back and the marks stay. Accepting alone leaves „Cofnij” disabled, and the accept toast has no „Cofnij”.

## Testing

To check in `npm run tauri dev`:

- Reject a photo, accept two others, Ctrl+Z → the rejected photo is back (still accepted if it was), and the two marks stay.
- Accepting or un-accepting alone leaves „Cofnij” disabled; the toast („Zaakceptowano …” / „Cofnięto akceptację …”) has no „Cofnij”.
- The hint of „Cofnij” reads „Przywróć ostatnio odrzucone zdjęcie (Ctrl+Z)”.
- macOS: reject one photo into „Odrzucone” and then one into the trash; Ctrl+Z brings back the one from „Odrzucone”. „Cofnij” then looks inactive; clicking it (or Ctrl+Z) shows „Na tym systemie nie można cofnąć przeniesienia do kosza – przywróć zdjęcie z Kosza ręcznie.”
