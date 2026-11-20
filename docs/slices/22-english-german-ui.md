---
status: 📝 planned
---

# Slice 22 – English and German UI

Brought forward on 2026-11-20 at the user's request, before [slice 15](15-settings.md); plan written and approved by the user the same day (all suggestions taken); next: implement step 1 of the plan.

## Idea

UI translations in English and German next to Polish (`en.json`, `de.json`), with dates and numbers formatted for the chosen language and correct plural rules. The language follows the system language by default and can be changed by the user; a test checks that every key exists in all three files. The rejected folder is named in the chosen language too (`Odrzucone` / `Rejected` / `Abgelehnt`); scans recognise all three names, so photos rejected under one language stay rejected after switching.

## Try

Start Sito on a system set to English, then German: every screen (main window, viewer, menus, dialogs, toasts, the update notice, the licenses) is in that language, with its dates, numbers and plurals („1 photo”, „2 photos”; „1 Foto”, „2 Fotos”). Switch the language in the menu while both windows are open: both change at once, without a restart. Reject a photo under English, switch to Polish, refresh: the photo stays hidden, the next rejection in that folder goes into the same `Rejected` folder, and „Cofnij” brings both back.

## Builds on

- `ui/src/i18n/index.ts` (i18next, one `pl` resource) and `pl.json` (about 200 keys, 3 of them with plural forms); `Intl` formatters fixed to `pl-PL` in `timeline/format.ts`, `viewer/photoDetails.ts`, `viewer/ZoomBadge.tsx`, `update/UpdateNotice.tsx`; `<html lang="pl">` in `ui/index.html`.
- `REJECTED_DIR` (`crates/core/src/scan.rs`), recognised in any letter case by the scan and the cached timeline; rejecting creates it in the photo's folder ([Rejecting](../domain/rejecting.md)).
- The native menu in `src-tauri/src/about.rs`, whose labels are Polish constants waiting for this slice.
- The NSIS installer with one language (Polish, own strings in `src-tauri/installer/Polish.nsh`).

## Concept

### Which language

- **Languages:** Polish, English, German. Any other system language gets English.
- **Default: the system language**, read on the Rust side (`sys-locale` crate, MIT OR Apache-2.0: `GetUserDefaultLocaleName` on Windows, the preferred languages on macOS). Not `navigator.language`: WKWebView on macOS reports the app bundle's localization (English unless the bundle lists others), not the user's language, and WebView2 can follow Edge's own language setting.
- **The user's choice** („Język systemu”, „Polski”, „English”, „Deutsch”; each language named in itself, „Język systemu” translated) is a setting kept by the backend, since Rust needs the language before the webview loads (the menu) and for every rejection (the folder name). It is stored in a new `settings` table (key → value) of `sito.db`, added by a numbered migration in `crates/core`; slice 15 (cache location, which cannot live in the cache) can keep its settings there too. The reject destination stays in `localStorage` as it is.
- **Switching** applies at once in both windows: the backend saves the choice, rebuilds the menu and tells every window (`language-changed`), which calls `i18n.changeLanguage` and sets `<html lang>`. No restart.

### Where the user switches it

There is no settings screen yet (slice 15 builds it). Recommended: a **„Ustawienia” menu with a „Język” submenu** of radio items, in the native menu next to „Pomoc” (Windows); on macOS in the app menu („Sito → Ustawienia → Język”), where settings live there. Slice 15 then adds „Ustawienia…” (the screen) to the same menu and may show the language there as well. The main window's bar stays as it is (it is full and one row at 900 px).

Alternative: only follow the system language, with no choice in the app. Simpler, but a user on a German system cannot get Polish.

### Dates and numbers

- Each language has a default locale: `pl-PL`, `en-GB`, `de-DE`. When the system locale has the same language as the UI (e.g. `en-US`, `de-AT`), the system locale is used instead, so an American user gets American dates and a Swiss user Swiss numbers. All three show 24-hour times.
- The `Intl` formatters fixed to `pl-PL` become per-locale (built once per locale, in one module of the i18n feature).
- Plurals: i18next already picks forms with `Intl.PluralRules`; English and German need `_one` / `_other`, Polish `_one` / `_few` / `_many` / `_other`.

### Texts

- `en.json` and `de.json` with every key of `pl.json`. Each language uses its own quotation marks („…” in Polish, “…” in English, „…“ in German) and its own name for the trash („Kosz”, “Recycle Bin” on Windows / “Trash” on macOS, „Papierkorb”).
- **A test** checks that the three files have the same keys, plural-aware: a key with plural forms has exactly the forms its language's `Intl.PluralRules` needs, and every `{{placeholder}}` of the Polish text appears in the others.
- **Missing keys never show raw:** in tests, i18next reports a missing key as a failure.
- **Wider texts:** German labels are often longer. The bar must stay one row at 900 px and the viewer's narrow-window rule is measured with its widest label („Cofnij akceptację”); both are checked again with the German and English labels, and wordings are shortened where they do not fit.
- **Keys stay the same in every language** (P, W, A, Delete, …), even though P and W are named after Polish words: one keymap, one key per action. The menus show the keys next to their entries.
- **The native menu's labels** („Pomoc”, „Licencje…”, „Sprawdź aktualizacje…”, the new „Ustawienia” / „Język”) live in a small table per language in `src-tauri` (the webview's translation files cannot reach a native menu), with a test that every label exists in all three.
- German texts are written by Claude; the user (or a native speaker) reviews them before the release.

### The rejected folder

- A rejection goes into a rejected folder in the photo's folder. Names: `Odrzucone`, `Rejected`, `Abgelehnt`. **Scans recognise all three** (in any letter case) in every language, so a rejected photo stays hidden whatever language it was rejected under.
- **A folder that already has a rejected folder keeps using it**, whatever its language: a rejection in a folder with `Odrzucone` goes there also under English. Only a folder without one gets a new folder in the current language. If a folder has several (e.g. created by the user), the one of the current language wins, else the first by name order. So switching the language never leaves two rejected folders side by side.
- **Existing folders are not renamed** on a language switch. This differs from the first idea (rename the folders Sito created), dropped because a rename on a settings change would touch the user's disk without a decision of theirs, can collide with sync clients and open Explorer windows, and needs every logged path rewritten; reusing the existing folder gives the same "one folder per photo folder" without moving anything. A user who wants the new name can rename the folder outside Sito: it is recognised under any of the three names, though „Cofnij” of a rejection whose folder was renamed reports the file as missing (as today).
- Core gets the names as a list (`REJECTED_DIR_NAMES`) and the folder to create as a parameter of the rejection, so `crates/core` stays without a notion of UI language; `src-tauri` maps the language to the name.
- Messages naming the folder („Folder „Odrzucone” jest skrótem…”, „…wybierz folder „Odrzucone”.”, the destination dropdown „folderu „Odrzucone””) name the current language's folder name.

### Installer and bundle

- **Windows installer:** English and German added next to Polish (NSIS picks the language of the system; no selector). Tauri's NSIS template ships English and German strings; check whether they cover our custom template's strings (`installer.nsi`), and add `English.nsh` / `German.nsh` like `Polish.nsh` where needed.
- **macOS:** the bundle lists its localizations (`CFBundleLocalizations`: pl, en, de), so macOS shows the app's own menu items (Hide, Quit, …) in the language and offers Sito in System Settings → Language & Region → Applications. Check on the Mac whether Tauri's default menu items follow it; if not, note it for [slice 23](23-macos-polish.md).
- The bundle's short and long description (`tauri.conf.json`) become English (they are not translated per language).

### Out of scope

- The release notes in „Co nowego?” stay English (they come from `CHANGELOG.md`, which is English); [slice 54](54-release-notes-markdown.md) formats them.
- The app name per language ([slice 53](53-rename-fotosito.md): PhotoSieve / FotoSito) is that slice's; this slice keeps the translation of the name in the translation files, so slice 53 only changes values.
- More languages: adding one is a new JSON file, a menu table entry and a rejected folder name.

## Plan

One commit per step, each with its tests and checks green.

1. **`feat`: the language setting.** `crates/core`: a `settings` module (migration adding `settings(key TEXT PRIMARY KEY, value TEXT NOT NULL)`, get/set, tests). `src-tauri`: `Language` enum (Polish, English, German) with the system language (`sys-locale`, license check, `notices:rust`, `cargo audit`) and the user's choice (`LanguageChoice`: system or a language); commands `get_language` (the effective language, the locale for formats and the choice) and `set_language`; the `language-changed` event to every window. Unit tests for mapping system locales (`pl-PL`, `en-US`, `de-AT`, `fr-FR` → English, empty) and choices.
2. **`refactor`: formatters per locale.** The four `pl-PL` formatters move into one i18n module that builds them per locale; tests in `pl-PL`, `en-GB`, `en-US` and `de-DE`.
3. **`feat`: English and German translations.** `en.json`, `de.json`; the i18n data layer (`ui/src/i18n/api.ts`) loads the language at start (before the first render, so nothing flashes in Polish) and follows `language-changed`; `<html lang>`. The key-completeness test; tests and the mocked Playwright suite stay in Polish, plus one mocked e2e test per language that walks the main screens with missing keys failing.
4. **`feat`: the language menu.** Menu label tables per language in `src-tauri`, „Ustawienia → Język” (radio items, the current choice checked), the menu rebuilt on a switch; test that every label exists in each language.
5. **`feat`: rejected folder per language.** `REJECTED_DIR_NAMES` in the scan and the cached timeline; the rejection takes the name to create and reuses an existing rejected folder (tests: none, one of each name, several, letter case, a symbolic link among them, undo in a reused folder, a nested folder). Messages name the folder of the current language.
6. **`build`: installer and bundle languages.** NSIS English and German, `CFBundleLocalizations`, English bundle descriptions.
7. **`docs`:** a new domain file `docs/domain/languages.md` (languages, the choice, formats, the rejected folder names), linked from the index and from [Rejecting](../domain/rejecting.md).

## Testing

- Unit and mocked e2e tests as above, also in this cloud container.
- In the real app (the user's machine): each language from the system setting and from the menu, switching with the viewer open, the bar at 900 px and the viewer at 824 px in German, plurals (0, 1, 2, 5, 22, 25 photos) and dates in each language, rejecting and undoing across a switch (a folder with `Odrzucone`, a folder without any, a folder with both `Odrzucone` and `Rejected`), the installer on an English or German Windows if available, the macOS app menu.

## Decisions of the review (2026-11-20)

- The language is chosen in the „Ustawienia → Język” menu until slice 15's settings screen exists.
- An existing rejected folder is reused, whatever its language; nothing is renamed on a switch.
- English chosen on a non-English system formats dates and numbers as `en-GB`.
