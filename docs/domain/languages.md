# Languages

Sito's UI is in Polish, English or German. This file says which one is shown, how the user changes it, how dates and numbers follow it and what it means for the rejected folder. Texts quoted elsewhere in these docs are the Polish ones.

## Which language

- **Default: the system language.** Polish and German systems get Polish and German; every other system language gets English. The language is read from the system by the app itself (on Windows the user's locale, on macOS the first preferred language), not from the webview, which on macOS reports the app's own language and on Windows can follow Edge's settings.
- **The user's choice** is made in the menu **„Ustawienia → Język”** (on macOS in the app menu „Sito”, after „About”; on Windows before „Pomoc”): „Język systemu”, „Polski”, „English”, „Deutsch”. Each language is named in itself, so it can be found whatever language is shown; the current choice is checked. The choice is kept across restarts (in the app's database next to the action log, not in the cache).
- **Switching** applies at once in both windows (open dialogs and toasts included) and in the menu, without a restart.
- Everything the app shows follows the language: both windows, menus, dialogs, toasts, the update notice and the licenses dialog. The release notes in „Co nowego?” stay English (they come from the changelog).
- Debug builds read the variable `SITO_SYSTEM_LOCALE` (e.g. `de-AT`) instead of the system's locale, to try a language without changing the system; the real-app tests set it to `pl-PL`.

## Dates and numbers

- Each language has a default locale: `pl-PL`, `en-GB`, `de-DE`. When the system locale has the same language as the UI (e.g. `en-US`, `de-AT`, `de-CH`), the system locale is used instead, so an American user gets American dates and a Swiss user Swiss numbers. English chosen on a Polish system formats as `en-GB`.
- Day headings („sobota, 1 czerwca 2024”, “Saturday, 1 June 2024”, „Samstag, 1. Juni 2024“), counts, percentages, file sizes and resolutions follow the locale. Times of day are always 24-hour `HH:mm:ss`, taken from the photo as it is.
- Plurals follow the language's rules: Polish „1 zdjęcie, 2 zdjęcia, 5 zdjęć”; English “1 photo, 2 photos”; German „1 Foto, 2 Fotos“.

## Texts

- Each language uses its own quotation marks („…” in Polish, “…” in English, „…“ in German) and its own name for the trash. The trash texts name the Windows Recycle Bin (“Recycle Bin”, „Papierkorb“); per-platform names on macOS come with [slice 23](../slices/23-macos-polish.md).
- Keyboard shortcuts are the same in every language (P, W, A, Delete, …), even where a key is named after a Polish word. Key names in hints follow the language („Entf“, „Strg+Z“, „Umschalt+F10“ in German).
- German labels are longer: the main window bar stays one row at its minimum width of 900 px, and the viewer keeps at least 300 px for the photo's details above its 824 px breakpoint, in every language. The German „Cofnij akceptację” is „Zurücknehmen“ to fit.
- The Windows installer is in English, Polish or German after the system's language (other languages get English); the macOS app lists Polish, English and German as its languages.

## The rejected folder

- A rejected photo goes into a rejected folder in its own folder ([Rejecting](rejecting.md)). Its name follows the language: `Odrzucone`, `Rejected` or `Abgelehnt`.
- **Scans recognise all three names** (in any letter case, at any depth) in every language, so a photo rejected under one language stays hidden after a switch.
- **A folder that already has a rejected folder keeps using it**, whatever its name: a rejection in a folder with `Odrzucone` goes there under English too. Only a folder without one gets a new one, in the current language. If a folder has several, the one of the current language is used, else the first by name (`Abgelehnt`, `Odrzucone`, `Rejected`). So switching the language never leaves two rejected folders side by side. A folder of another letter case (`rejected`) is reused on Windows and macOS, whose file systems ignore case.
- Existing rejected folders are never renamed on a switch. A user who wants another name can rename the folder outside Sito; it is still recognised, though „Cofnij” of a rejection made before the rename reports the file as missing.
- Messages about the folder name the current language's folder („Folder „Odrzucone” jest skrótem…”, “The “Rejected” folder is a shortcut…”), also when the folder in question has another of the three names.
