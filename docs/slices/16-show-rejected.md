# Slice 16 – Show and restore rejected photos

**Status:** ⬜ not started (moved from the roadmap; concept and plan when the slice starts)

## Idea

Option to show photos from `Odrzucone` folders in the timeline and the viewer, visually weaker (faded, marked "odrzucone") so they do not get in the way. A rejected photo can be restored: moved back from `Odrzucone` to its folder (logged and undoable).

## Try

Show rejected photos, check they are faded but still browsable, restore one and see it back in its folder.

## Notes from slice 34

Photos rejected into the system trash ([slice 34](34-reject-to-trash.md)) are not in any `Odrzucone` folder, so this slice will not show them; restoring those is „Cofnij” or the system trash itself. Restoring a photo from `Odrzucone` can log its move like the existing folder undo (`crates/core/src/reject.rs`, `restore`).

## Notes from slice 32

Accepted photos ([slice 32](32-accept-photo.md)) are stored by path in the `accepted` table of the action log, read with `ActionLog::accepted_paths(under)` (`crates/core/src/accept.rs`). A view of accepted photos could be built the same way as the rejected one here. Undoing a rejection keeps an accepted photo accepted, since the mark stays with its original path.
