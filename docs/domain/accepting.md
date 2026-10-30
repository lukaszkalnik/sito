# Accepting

How the user marks the keepers of a series. Sito only suggests; accepting is the user's own decision and never moves or changes a file.

- **„Zaakceptuj”** (green, with a check) in the viewer bar, left of „Odrzuć”, or the key **A**, marks the shown photo as accepted. The viewer stays on the photo, so several photos of one series can be accepted in a row.
- On an accepted photo the button is pressed and reads **„Cofnij akceptację”**; it and A take the mark off again. Holding A does not repeat (once per press, like Delete).
- A toast confirms each change, „Zaakceptowano <name>” or „Cofnięto akceptację <name>”, with „Cofnij”.
- Accepted photos carry a **green check** in the bottom-right corner of their grid tiles and of their thumbnails in the viewer's series strip (not on the big photo). Its label is „Zaakceptowane”.
- The mark is kept by the photo's **path**, so it survives restarts and rescans. A photo moved or renamed outside Sito loses its mark.
- A rejected accepted photo keeps its mark: undoing the rejection brings it back accepted.

## Undo

- „Cofnij” (and Ctrl+Z) is **one stack per open folder, newest first**, for rejections ([Rejecting](rejecting.md)), accepts and un-accepts alike, also after a restart.
- Undoing an accept takes the mark off („Cofnięto akceptację <name>”); undoing an un-accept puts it back („Przywrócono akceptację <name>”). The window then shows (viewer) or scrolls to and highlights (grid) the photo, as after a restored rejection.
- Accepting a photo that already is accepted (e.g. from a second window) changes nothing and adds nothing to undo.
