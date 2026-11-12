# Accepting

How the user marks the keepers of a series. Sito only suggests; accepting is the user's own decision and never moves or changes a file.

- **„Zaakceptuj”** (green, with a check; only the check in a narrow viewer window, see [Viewer](timeline.md#viewer)) in the viewer bar, left of „Odrzuć”, or the key **A**, marks the shown photo as accepted. The viewer stays on the photo, so several photos of one series can be accepted in a row.
- On an accepted photo the button is pressed and reads **„Cofnij akceptację”**; it and A take the mark off again. Holding A does not repeat (once per press, like Delete).
- A toast confirms each change, „Zaakceptowano <name>” or „Cofnięto akceptację <name>”, without „Cofnij”: the button and A take a mark back.
- Accepted photos carry a **green check** in the bottom-right corner of their grid tiles and of their thumbnails in the viewer's series strip (not on the big photo). Its label is „Zaakceptowane”.
- The mark is kept by the photo's **path**, so it survives restarts and rescans. A photo moved or renamed outside Sito loses its mark.
- A rejected accepted photo keeps its mark: undoing the rejection brings it back accepted.

## Undo

- „Cofnij” (and Ctrl+Z) is only for rejections ([Rejecting](rejecting.md)); it never changes a mark. Accepting and un-accepting are not logged, so they neither enable „Cofnij” nor stand between it and the newest rejection.
- Logs of older versions (slices 32–42) may still hold logged accepts and un-accepts; undo ignores them.
- Accepting a photo that already is accepted (e.g. from a second window) changes nothing.
