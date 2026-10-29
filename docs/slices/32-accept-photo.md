# Slice 32 – Accept a photo

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

Split out of [slice 12](12-user-feedback-1.md) (item 3) on 2026-10-29: it needs its own storage and undo, unlike the quick viewer fixes there. From the user's first real test (2026-10-27).

## Concept

- **„Zaakceptuj”** marks a photo as accepted, e.g. as the first pick in a large series.
- Button and key (to decide: `A` or `Enter`).
- A green check mark appears on the photo's **thumbnails only** (timeline and series strip), not on the photo in the viewer.
- Stored in SQLite next to the action log, so it survives restarts, and undoable like a rejection ([Rejecting](../domain/rejecting.md)).

## Open questions

- May several photos of a series be accepted?
- Does accepting the last one move on to the next series?

## Related slices

- [Slice 16](16-show-rejected.md) (rejected photos as a view) could show accepted photos the same way.

## Verification (outline)

- The best shot of a big series can be accepted; its check mark shows in the timeline and in the strip, and survives a restart.
- „Cofnij” takes the acceptance back.
