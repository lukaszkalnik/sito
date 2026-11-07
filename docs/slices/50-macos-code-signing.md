---
status: 📝 concept drafted
---

# Slice 50 – macOS code signing and notarization

Since 2026-11-07; plan in plan mode when the slice starts. Split out of [slice 23](23-macos-polish.md), which keeps the other macOS fixes.

**Postponed** by the user on 2026-11-07 until after the 1.0 release (more features, at least English and German UI, [slice 22](22-english-german-ui.md)): the Apple Developer Program costs 99 USD a year, worth paying only once Sito is ready for a wider audience. Until then the unsigned DMG stays, with the Privacy & Security steps in the release notes.

## Idea

Asked by the user on 2026-11-07: the macOS app is signed with a Developer ID certificate and notarized by Apple, so it opens like any downloaded app, with at most the usual "downloaded from the internet" question.

**Why now:** installing v0.x on the user's Mac was hard to figure out. macOS refused to start the unsigned app, and the right-click → *Open* bypass in [Releasing](../RELEASING.md#installing-on-macos) no longer works on current macOS (removed in macOS 15 Sequoia). The only way was System Settings → Privacy & Security, scrolling down past many entries to *Open Anyway*. Testers and other users will not find that.

This is the Developer ID path (download from GitHub or the [promo website](49-promo-website.md)). The Mac App Store is a different certificate and a sandboxed build, see [slice 51](51-mac-app-store.md). It is separate from the minisign update signing of [slice 41](41-update-check.md), which stays as it is.

## What it needs

- **Apple Developer Program membership** (99 USD a year, individual membership is fine; the app then shows the user's own name as the developer). Only the user can enrol: Apple ID with two-factor authentication, identity check. Enrolment can take from a few hours to a few days, so it is the first user step.
- **Developer ID Application certificate**, created in the developer account, exported as a `.p12` with a password. Unlike Windows (see [slice 47](47-windows-code-signing.md)), Apple still allows a key file in CI, so it can go into GitHub secrets.
- **Notarization credentials:** an App Store Connect API key (preferred: not tied to the user's Apple ID password) or an Apple ID with an app-specific password and the team ID.

## How Tauri does it

`tauri build` (and `tauri-action`) signs and notarizes when these environment variables are set: `APPLE_CERTIFICATE` (base64 `.p12`), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, and for notarization `APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH` (or `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`). It signs with the hardened runtime, submits to the notary service, waits for it and staples the ticket. Check when planning whether the DMG itself also gets signed, notarized and stapled, and add the steps if not.

## To check when planning

- **Entitlements:** whether the hardened runtime needs any for WKWebView, the trash ([slice 34](34-reject-to-trash.md)) or later the ONNX Runtime dylib (planned `ort`, [slice 19](19-split-by-content.md), [21](21-faces-and-eyes.md)); every bundled dylib must be signed too. Keep the list minimal, each with a reason (CLAUDE.md "Security and privacy").
- **Universal build:** both architectures in one signed bundle; notarization covers both.
- **Updater on macOS:** with a signed app, enable installing from the notice on macOS (today it says to download the DMG, see [Updates](../domain/updates.md#installing)). The updater replaces the `.app` from the signed `.app.tar.gz`; check that the replaced app passes Gatekeeper, keeps its permissions (folder access prompts) and restarts. Check the first update from an unsigned to a signed version.
- **Release workflow:** sign and notarize only on tag builds, or also on manual runs (notarization takes minutes)? Add a check before publishing: `codesign --verify --deep --strict`, `spctl --assess --type execute` on the app and `--type open --context context:primary-signature` on the DMG, `xcrun stapler validate`. Build it next to the Windows check of [slice 47](47-windows-code-signing.md) so both share one pattern. Remove the "signing comes later (slice 23)" comment in `.github/workflows/release.yml`.
- **Secrets and backup:** user steps (enrolment, certificate, API key) written down as with the update key in slice 41. The `.p12`, its password and the API key go into 1Password as well; renewal (Developer ID certificates last five years, the membership one year) goes into `docs/RELEASING.md`. If the membership lapses, signed releases keep working, but no new ones can be notarized.
- **Docs:** `docs/RELEASING.md` "Installing on macOS" (no bypass steps any more), CLAUDE.md "CI" ("Unsigned for now; macOS signing and notarization come later"), `docs/domain/updates.md`.

## Until then

[Releasing](../RELEASING.md#installing-on-macos) describes the Privacy & Security path that works on current macOS, so the next unsigned release is at least installable by following the notes.

## Related slices

- [Slice 47](47-windows-code-signing.md): the same work for Windows; shared release workflow pattern.
- [Slice 51](51-mac-app-store.md): Mac App Store; reuses the developer account and the bundle ID.
- [Slice 42](42-zero-click-update.md): silent updates on macOS need this slice.

## Try

Download the DMG of a signed release in Safari, open it, drag Sito into Applications and start it: macOS asks at most "Sito is an app downloaded from the internet. Are you sure you want to open it?" and shows no block. `spctl -a -vv /Applications/Sito.app` reports `source=Notarized Developer ID`. An update from the notice installs and restarts Sito.
