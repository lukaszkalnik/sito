---
status: 📝 concept drafted
---

# Slice 41a – Link to the release page in the update notice

Since 2026-11-08; plan in plan mode when the slice starts.

## Idea

Found by the user on 2026-11-08 while testing [slice 41](41-update-check.md) on the Mac: the notice says „Dostępna jest nowa wersja Sito: 0.15.1. Pobierz ją ze strony wydań Sito na GitHubie.” but gives no way to get there; the user has to find the page themselves. Slice 41 left the link out on purpose, since opening the browser needs a new capability ([Updates](../domain/updates.md), „macOS”).

The macOS notice gets a button (e.g. „Pobierz”) that opens the release page of the offered version (`https://github.com/lukaszkalnik/sito/releases/tag/vX.Y.Z`) in the system browser. It is the macOS counterpart of „Zainstaluj i uruchom ponownie” and stays until in-app install comes with signing ([slice 50](50-macos-code-signing.md)).

## Candidate approaches

- **A Rust command that builds the URL itself (preferred).** `open_release_page(version)` validates the version, builds the GitHub URL from a constant and opens it with the system opener (`tauri-plugin-opener`'s Rust API or the `open` crate). The UI never passes a URL, so no capability lets the webview open arbitrary addresses.
- **The opener plugin's JS API**, scoped in the capability to `https://github.com/lukaszkalnik/sito/releases/*`. Less Rust, but a webview permission.

Either way, CLAUDE.md asks for a stated reason for the new permission, and the dependency's license goes through the usual review (`about.toml`, `npm run notices:rust`).

## Related

- [Slice 54](54-release-notes-markdown.md) (formatted release notes) may want clickable links in the notes; build the opener so it can take a checked GitHub URL later, not only the release page.
- [Slice 50](50-macos-code-signing.md): with signing, macOS installs in the app and this button goes away.
