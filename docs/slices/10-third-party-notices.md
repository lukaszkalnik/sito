# Slice 10 – Third-party license notices

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

Sito itself is licensed MIT OR Apache-2.0 (since 2026-10-27). The installer, however, contains compiled copies of many libraries:
- **Rust crates**, e.g. `tauri`, `image`, `rusqlite` (with SQLite, public domain), `walkdir`
- **npm packages** bundled into the UI, e.g. React, i18next, TanStack Virtual

Their licenses (mostly MIT, Apache-2.0, BSD, ISC, zlib) require their copyright and license notice to ship with every copy, binaries included. The repo itself doesn't need them, because Cargo and npm fetch them with their own license files. Today the installer ships no notices.

## Concept

- **Rust crates:** `cargo-about` generates the notices from `Cargo.lock`, using a template and an `about.toml` that lists the accepted licenses.
- **npm packages:** a Vite plugin (e.g. `rollup-plugin-license`) writes the notices of exactly the packages that end up in the UI bundle.
- **One file:** both parts are combined into `THIRD-PARTY-NOTICES.txt` (or HTML) at build time.
- **In the installer:** the file is bundled as a Tauri resource. A „Licencje” link in the app opens it in a simple window or view. It sits next to the version number in the „O programie” entry planned in slice 11.
- **Checked in CI:** CI fails if a dependency brings a license that isn't on the reviewed list (`cargo-about` / `cargo-deny` for Rust, the plugin's allow-list for npm), so nothing slips in unnoticed.
- **Also shown:** Sito's own license (MIT OR Apache-2.0).

## Open questions

- A plain text file, or a formatted HTML page inside the app?
- `cargo-about` or `cargo-deny` (or both) for the license check in CI?
- Does `cargo-about` install quickly enough in CI, or is a cached binary needed?

## Verification (outline)

- `THIRD-PARTY-NOTICES` contains React and `image` with their full license texts.
- The installed app opens it from „Licencje”.
- CI fails for a test dependency with a license that isn't on the list.
