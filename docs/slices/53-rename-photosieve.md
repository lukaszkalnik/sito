---
status: 📝 concept drafted
---

# Slice 53 – Rename to PhotoSieve

Since 2026-11-07; plan when the slice starts. Renamed from „Rename to FotoSito / PhotoSieve” on 2026-11-22, when the user chose one English name for every language ([Decisions](#decisions)).

## Idea

Asked by the user on 2026-11-07: the app gets a new user-facing name, **PhotoSieve**, the same in every UI language (Polish, English, German) and everywhere the name appears: window titles, the in-app texts, the installer, Start menu and Apps & features, the macOS `.app`, GitHub releases, store listings and the promo website.

## Decisions

Decided by the user on 2026-11-22:

1. **One English name, PhotoSieve, for every language.** The per-language names first considered (FotoSito in the Polish UI, FotoSieb in the German one) are dropped. Reasons: the system-level name (`productName`: installer, install folder, Start menu, Apps & features, macOS `.app`, the updater's file names) is one name anyway, so per-language names would show different names in the window and in the Start menu; the UI language can be changed at any time ([slice 22](22-english-german-ui.md)), and the app should not change its name with it; one name is found in search, the stores and on the website, and screenshots and support questions do not depend on the language.
2. **The name is a proper noun, not translated.** It still lives in one place, not hardcoded in components: the `app.title` key of the translation files, with the same value in `pl.json`, `en.json` and `de.json` (or one shared constant the translations use; decide when planning). The taglines stay translated and may carry the sieve idea in each language („Przesiewa serie zdjęć…”).
3. **„Sito” stays the code name** (repository, crates, identifiers, the bundle identifier, `CLAUDE.md` and the internal docs), as below.

**Only user-facing text and promotional materials change.** The GitHub repo (`lukaszkalnik/sito`), the crate and package names (`sito`, `sito-core`, `ui`), Rust/TS identifiers, the bundle identifier `io.github.lukaszkalnik.sito`, `CLAUDE.md` and the internal docs keep "Sito" as the project's code name.

## What changes (found on 2026-11-07)

- **UI strings:** `ui/src/i18n/pl.json` (`app.title`, the licenses intro, the update notice texts); the matching tests (`App.test.tsx`, `UpdateNotice.test.tsx`, `LicensesDialog.test.tsx`, `PhotoViewer.test.tsx`).
- **Hardcoded window titles:** `ui/index.html` `<title>`, `src-tauri/src/viewer.rs` (`.title("Sito")`), the viewer title `` `${entry.name} – Sito` `` in `PhotoViewer.tsx`, and `"title"` in `src-tauri/tauri.conf.json`. The app name should come from one place (the `app.title` key, the same value in all three translation files, see [Decisions](#decisions)); the Rust-side initial titles need a value that is replaced once the UI loads, or the name passed from the UI.
- **Bundle metadata:** `productName`, `shortDescription`, `longDescription` in `tauri.conf.json` (installer, Start menu entry, macOS app name, Apps & features entry).
- **Third-party notices text:** "Sito is licensed under…", "bundled into Sito's user interface" in `ui/build/thirdPartyNotices.ts` (shown in the app under „Pomoc → Licencje…”).
- **Release materials:** `releaseName: 'Sito …'` in `.github/workflows/release.yml`, the install instructions in `scripts/release-notes.mjs`, `CHANGELOG.md` from the release that ships the rename on (older sections stay as they were).
- **Promotional materials:** `README.md` (title, pitch, image alt texts), the README screenshots in `docs/images/` if they show the old name (window title), [slice 49](49-promo-website.md) (promo website), store listings in [slice 51](51-mac-app-store.md) / [slice 52](52-microsoft-store.md).
- **User-facing docs:** `docs/RELEASING.md` install and uninstall steps (app name in Start menu, Apps, Applications, Gatekeeper dialog).

## To check when planning

- **`productName` becomes PhotoSieve** ([Decisions](#decisions)). It drives the installer file names (`Sito_X.Y.Z_x64-setup.exe` → `PhotoSieve_…`), the install folder (`%LOCALAPPDATA%\Sito`), the Start menu entry and the macOS `.app` name; see the update path below.
- **Updating an installed Sito.** A new `productName` may make the NSIS installer install side by side into a new folder instead of updating `%LOCALAPPDATA%\Sito`, and leave the old Start menu entry and uninstall entry behind. Check what Tauri's NSIS template keys the install dir and registry entries on (`productName` vs. `identifier`), and test an update from the newest released version in the smoke test or by hand. On macOS, an old `Sito.app` next to a new `PhotoSieve.app` in Applications.
- **App data must survive.** The photo cache, the action log (undo history) and the window state live in directories derived from the `identifier`, which does not change; confirm that, including the `window-state` plugin and the WebView data (`localStorage` with the per-folder grid positions, [slice 40](40-keep-grid-position.md)).
- **Updater:** `latest.json` and the asset name patterns ([Updates](../domain/updates.md), `scripts/check-update-manifest.mjs`, `scripts/local-update-manifest.mjs`) must match the new installer names; an installed old version must still find and install the first renamed release.
- **Release workflow smoke test:** the install path `Join-Path $env:LOCALAPPDATA 'Sito'` in `release.yml` follows the new install folder.
- **Icon:** check that the icon (`scripts/render-icon.mjs`) carries no letters of the old name.
- **Rejected folder name** („Odrzucone”) is not the app name and stays.
- **Domain docs:** describe the names in a short section (e.g. `docs/domain/README.md` or a new `naming.md`), and the CLAUDE.md intro line can mention that "Sito" is the code name of PhotoSieve.

## Related

- [Slice 22](22-english-german-ui.md) (English and German UI): keeps the name in the translation files (`app.title`); this slice sets it to PhotoSieve in all three.
- [Slice 49](49-promo-website.md) (promo website), [slice 51](51-mac-app-store.md), [slice 52](52-microsoft-store.md): use the new name from the start, so this slice should come before them.
