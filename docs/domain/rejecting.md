# Rejecting

- A rejected photo is moved into an `Odrzucone` subfolder of its own folder (Polish for "rejected"); it is never deleted. Settings for the system trash instead are planned (slice 15).
- A name that already exists there is kept: the photo gets `<name> (2)`, `(3)` and so on.
- **Every file of a photo moves together:** the RAW file of a RAW+JPEG pair goes along with the JPEG and keeps the same number if names are taken (`P1 (2).JPG` + `P1 (2).ORF`); undo brings all of them back.
- A photo's own Google Takeout date file moves along into `Odrzucone`, named after the photo (`<photo name>.supplemental-metadata.json`), and back on undo, so the photo keeps its date and no date file is left behind. The date file moves first; if the photo then cannot move, the date file goes back. A date file shared with other photos (an edited copy uses its original's) stays where it is.
- Every rejection can be undone ("Cofnij"), newest first, also after a restart. Undo puts the photo back under its original name and refuses if another file has taken that name meanwhile.
- Photos in `Odrzucone` folders are hidden from the timeline; slice 16 shows them again.
