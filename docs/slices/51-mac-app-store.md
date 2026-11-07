---
status: 📝 concept drafted
---

# Slice 51 – Mac App Store

Since 2026-11-07; plan in plan mode when the slice starts.

**Postponed** by the user on 2026-11-07 until after the 1.0 release, together with [slice 50](50-macos-code-signing.md) (same paid developer account).

## Idea

Asked by the user on 2026-11-07: Sito is available in the Mac App Store, next to the download from GitHub ([slice 50](50-macos-code-signing.md)). As on Windows ([slice 52](52-microsoft-store.md)), the store is the main way to install and the [promo website](49-promo-website.md) links to it prominently; the GitHub download is the second option. Users find it there, install it with one click, and the store keeps it up to date.

## What is different from the Developer ID build

- **App Sandbox is required.** This is the main work. Sito may only touch files the user granted:
  - A folder picked in the dialog is accessible for that session. Reopening the last folder on start ([slice 9](09-persistence.md)) and keeping the grid position per folder ([slice 40](40-keep-grid-position.md)) need **security-scoped bookmarks**, stored with the folder and resolved on start (entitlement `com.apple.security.files.bookmarks.app-scope`, `files.user-selected.read-write`).
  - Rejecting into „Odrzucone” or the trash ([slice 8](08-reject-with-undo.md), [slice 34](34-reject-to-trash.md)) and undo must work inside the granted folder; check the trash from a sandbox.
  - The photo cache, thumbnails and action log move into the app container (`~/Library/Containers/<bundle id>/`); they are caches or the user's undo history, so decide whether a user switching from the GitHub build keeps the log.
  - The model download (later, `ort`) needs `com.apple.security.network.client`; nothing else goes online.
- **No in-app updater.** The store updates the app; the store build turns off the update check of [slice 41](41-update-check.md) (a build flag or Cargo feature, not a runtime setting), and the „Sprawdź aktualizacje” menu entry is hidden.
- **Other certificates:** Apple Distribution and Mac Installer Distribution, plus a Mac App Store provisioning profile. The build is a signed `.pkg` uploaded to App Store Connect (Transporter, or `xcrun altool` in CI). Tauri documents this flow.
- **Review and listing:** App Review, a privacy label (Sito collects no data), a support URL (the [promo website](49-promo-website.md)), screenshots ([slice 11](11-readme-showcase.md)), a Polish (and English) store description, price (free?), age rating, and export compliance (no own encryption).

## To check when planning

- Whether one codebase with two build profiles (Developer ID / App Store) stays simple, and which code paths differ (bookmarks, updater). Bookmarks also work outside the sandbox, so using them in both builds may keep a single path.
- Whether Apple accepts the app: Tauri apps are in the store, but check the hardened runtime and entitlements from slice 50 against the sandbox.
- Version numbering: the store needs a build number that increases per upload.
- Whether the release workflow uploads to App Store Connect on every tag or the user submits by hand.
- Licenses: third-party notices stay reachable in the app (already the case).

## Builds on

- [Slice 50](50-macos-code-signing.md): the developer account, team, bundle ID and the signing setup in CI.

## Related slices

- [Slice 52](52-microsoft-store.md): the same for Windows; both turn off the in-app updater the same way.

## Try

Install Sito from the Mac App Store (TestFlight for Mac first), open a folder, reject and undo a photo, quit and start again: the last folder opens without asking again. No update notice appears; a new version arrives through the store.
