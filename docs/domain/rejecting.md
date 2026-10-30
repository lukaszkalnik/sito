# Rejecting

- **Where a rejected photo goes** is chosen in the main window, before „Cofnij”: „Odrzucone zdjęcia do: [folderu „Odrzucone” | kosza]”. The default is the folder; the choice is remembered across restarts and shared by the main window and the viewer.
  - Choosing „kosza” asks first („Przenosić odrzucone zdjęcia do kosza?”; „Anuluj” has the focus and Escape cancels). Going back to the folder does not ask.
  - While the trash is chosen, the control (and its trash icon) is red and bold, so it is never chosen unnoticed. In the viewer, „Odrzuć” shows the same folder or trash icon before its text, and its hint and the toast („<name> → Kosz”) follow the choice; the icon changes as soon as the choice changes in the main window.
- **Folder:** a rejected photo is moved into an `Odrzucone` subfolder of its own folder (Polish for "rejected"); it is never deleted.
  - A name that already exists there is kept: the photo gets `<name> (2)`, `(3)` and so on.
  - A photo's own Google Takeout date file moves along into `Odrzucone`, named after the photo (`<photo name>.supplemental-metadata.json`), and back on undo, so the photo keeps its date and no date file is left behind. The date file moves first; if the photo then cannot move, the date file goes back.
- **Trash:** the photo and its files go into the system trash (the Recycle Bin on Windows) in one system operation. The trash keeps them, so this is still a move, not a delete; they can be restored with „Cofnij” or from the trash itself until it is emptied.
  - Only drives that have a trash qualify: on removable drives (USB sticks, SD cards) and network drives Windows would delete the files for good, so rejecting into the trash is refused there with „Ten dysk nie ma kosza – wybierz folder „Odrzucone”.” Nothing is moved.
  - A photo in the trash is no longer in its folder, so after a rescan it no longer counts for its series (unlike a photo in `Odrzucone`).
- **Every file of a photo moves together**, in both cases: the RAW file of a RAW+JPEG pair goes along with the JPEG (in `Odrzucone` it keeps the same number if names are taken: `P1 (2).JPG` + `P1 (2).ORF`), and so do their own date files. A date file shared with other photos (an original and its edited copy) stays where it is.
- Every rejection can be undone („Cofnij”), newest first, also after a restart, whichever destination it went to. „Cofnij” is one stack shared with accepting photos ([Accepting](accepting.md)). Undo puts the photo back under its original name and refuses if another file has taken that name meanwhile.
  - From the trash, undo takes the newest file deleted from that path since the rejection; if it is no longer in the trash: „plik nie jest już w Koszu”, and the next „Cofnij” goes on with the rejection before.
  - macOS has no restore API for the trash yet: undo of a trash move says so and is skipped; the photo can be put back from the Trash by hand (see [slice 23](../slices/23-macos-polish.md)).
- Photos in `Odrzucone` folders are hidden from the timeline; slice 16 shows them again.
