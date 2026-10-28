# Slice 22 – English and German UI

**Status:** ⬜ not started (moved from the roadmap; concept and plan when the slice starts)

## Idea

UI translations in English and German next to Polish (`en.json`, `de.json`), with dates and numbers formatted for the chosen language (`en-GB`/`de-DE`) and correct plural rules. The language follows the system language by default and can be changed in settings; a test checks that every key exists in all three files. The rejected folder is named in the chosen language too (`Odrzucone` / `Rejected` / `Abgelehnt`); scans recognise all three names, so photos rejected under one language stay rejected after switching. After a language switch, rejected folders that Sito created itself (recorded in the action log when created) are renamed to the new name, and the log's paths are updated so undo keeps working; folders created by the user keep their name, and a rename is skipped if a folder with the new name already exists.

## Try

Switch the system language (or the setting) to English and German and check all screens, plurals and dates.
