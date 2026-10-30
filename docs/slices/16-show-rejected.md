# Slice 16 – Show and restore rejected photos

**Status:** ⬜ not started (moved from the roadmap; concept and plan when the slice starts)

## Idea

Option to show photos from `Odrzucone` folders in the timeline and the viewer, visually weaker (faded, marked "odrzucone") so they do not get in the way. A rejected photo can be restored: moved back from `Odrzucone` to its folder (logged and undoable).

## Try

Show rejected photos, check they are faded but still browsable, restore one and see it back in its folder.

## Notes from slice 34

Photos rejected into the system trash ([slice 34](34-reject-to-trash.md)) are not in any `Odrzucone` folder, so this slice will not show them; restoring those is „Cofnij” or the system trash itself. Restoring a photo from `Odrzucone` can log its move like the existing folder undo (`crates/core/src/reject.rs`, `restore`).
