---
status: 📝 concept drafted
---

# Slice 52 – Microsoft Store

Since 2026-11-07; plan in plan mode when the slice starts.

## Idea

Asked by the user on 2026-11-07: Sito is available in the Microsoft Store, next to the installer from GitHub. Users find it there, install it without SmartScreen warnings, and the store keeps it up to date.

**The store is the main way to install on Windows** (user, 2026-11-07). The [promo website](49-promo-website.md) links to it prominently; the unsigned NSIS installer from GitHub stays as a second option for people who avoid the store, and its SmartScreen steps are acceptable for them. This replaces code signing the installer ([slice 47](47-windows-code-signing.md), parked).

## Candidate approaches

1. **MSIX package.** The store re-signs MSIX packages with a Microsoft certificate, so the store copy needs no own code-signing certificate. This is relevant to [slice 47](47-windows-code-signing.md): for an individual in Germany, signing is hard to get (see there), and the store is a way to give Windows users a trusted install without it. It does not sign the GitHub download. Tauri does not build MSIX itself; the `.exe` from `tauri build` would be packed with `makeappx`/MSIX Packaging Tool or `winapp` in CI, with an `AppxManifest.xml`. MSIX apps run in a light container: file system writes to `%LOCALAPPDATA%` are redirected, which matters for the photo cache and action log locations, and full file system access to photo folders needs `runFullTrust` (a restricted capability that review must accept, common for desktop apps).
2. **Submit the existing NSIS installer** (the store accepts Win32 `.exe`/`.msi`). Least packaging work, and the in-app updater could even stay. But the store requires the installer to be signed with a trusted certificate, so it depends on slice 47, and it must install silently with a fixed switch (`/S`).

**Chosen: 1** (follows from the user's decision of 2026-11-07 to park slice 47): only MSIX gets a trusted signature without a certificate of our own.

## What it needs

- **Partner Center developer account.** Individual accounts are free (to confirm when planning; it used to cost about 19 USD once). Identity check by the user.
- **Store listing:** Polish (and English) description, screenshots ([slice 11](11-readme-showcase.md)), privacy policy URL (the [promo website](49-promo-website.md)), age rating, price.
- **No in-app updater** in the MSIX build: the store updates the app. Turn off the update check of [slice 41](41-update-check.md) the same way as the Mac App Store build of [slice 51](51-mac-app-store.md) (a build flag, not a runtime setting).

## To check when planning

- File access from MSIX: picking any folder, moving files to „Odrzucone” and the recycle bin ([slice 34](34-reject-to-trash.md)), long and Polish paths.
- Where the cache and action log end up under MSIX redirection, and what happens to them when the user uninstalls (MSIX removes the app's data).
- WebView2: present on Windows 10/11; the MSIX does not need to bundle it.
- Whether the release workflow uploads to Partner Center on every tag (Microsoft Store submission API / `msstore` CLI) or the user submits by hand.
- **Who tests it:** the user has no Windows machine; the store's own certification runs the app, and someone with Windows installs it from the store.

## Related slices

- [Slice 47](47-windows-code-signing.md): parked, since the store copy is trusted without it.
- [Slice 49](49-promo-website.md): store badge as the main download button.
- [Slice 51](51-mac-app-store.md): same work for macOS, shared updater switch.
- [Slice 35](35-install-anywhere.md): the store decides the install location, so that slice applies only to the NSIS installer.

## Try

Install Sito from the Microsoft Store on Windows, open a folder, reject and undo a photo, restart: it behaves as the GitHub build, with no SmartScreen warning and no update notice. A new version arrives through the store.
