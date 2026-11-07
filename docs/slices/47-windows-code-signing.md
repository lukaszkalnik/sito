---
status: 📝 concept drafted
---

# Slice 47 – Windows code signing

Since 2026-11-07; plan in plan mode when the slice starts.

## Idea

Asked by the user on 2026-11-07: the Windows installer and app are code-signed, so Windows shows a known publisher instead of "Unknown publisher" and SmartScreen stops blocking the download and the first start ("Windows protected your PC" → *More info* → *Run anyway*, see [Releasing](../RELEASING.md#installing-on-windows)).

[Slice 2](02-installable-app.md) left code signing out because of its cost. This is separate from the minisign update signing of [slice 41](41-update-check.md), which stays as it is.

## What gets signed

- `sito.exe` (the app), the NSIS installer `Sito_X.Y.Z_x64-setup.exe` and the `uninstall.exe` it writes. Tauri signs all of them during `tauri build` when `bundle.windows` has a certificate or a `signCommand`. It signs before it writes the update signature (`.sig`), so `latest.json` covers the code-signed installer.
- Signatures carry an RFC 3161 timestamp (`bundle.windows.timestampUrl`), so they stay valid after the certificate expires.

## Candidate approaches

Since June 2023, code-signing keys must live on a hardware token or a cloud HSM. A `.pfx` file in a GitHub secret is no longer possible, so signing in CI always goes through a signing service.

1. **SignPath Foundation (free for open source).** Sito is public and MIT/Apache-2.0, which may qualify it. CI uploads the unsigned files to SignPath through its GitHub Action, and they come back signed. The certificate is issued to "SignPath Foundation", not to Lukasz Kalnik, and the project has to meet their rules (an OSI license, a public repo, builds only from CI, a release approval step). It fits Tauri less well: Tauri signs the inner `sito.exe` before it packs the installer, so this would need a `signCommand` that calls SignPath, or two build passes.
2. **Azure Artifact Signing (formerly Trusted Signing), about 10 USD a month.** Microsoft's own cloud signing, with the certificate in the user's name and fast SmartScreen reputation. Tauri documents it: `signCommand` with `trusted-signing-cli`, and an Azure app registration whose credentials go into GitHub secrets. Check whether individual developers outside the USA and Canada can verify their identity yet; that is what limited it so far.
3. **Certum Open Source Code Signing (Polish CA, cheap for open source).** An individual OV certificate on SimplySign, Certum's cloud HSM. Automating it in CI is awkward: SimplySign needs a login with a one-time code from the phone app, so it may only work with a TOTP secret in CI or with signing by hand.
4. **A standard OV or EV certificate from a CA, on a token or cloud HSM:** the most expensive. Since 2024, EV no longer gets instant SmartScreen trust, so it buys nothing over OV here.

The first choice is 2 if the user is eligible (publisher in their own name, documented Tauri path), otherwise 1 (free, but a different publisher name). The user decides when planning, since it involves cost and an identity check that only they can do.

## To check when planning

- **SmartScreen reputation:** an OV signature does not end the warning at once; reputation builds up with downloads of files signed with the same certificate. Find out what each option gives on the first release, so the changelog and `docs/RELEASING.md` promise only what is true.
- **Secrets and setup:** steps only the user can take (account, identity check, certificate) are written down as a user step, as with the update key in slice 41. Credentials go into repository secrets, and a backup or recovery path goes into `docs/RELEASING.md`.
- **Release workflow:** sign only on tag builds, or also on manual runs? Every signature costs quota and approvals, so test builds may stay unsigned. Add a check before publishing that fails the release when the installer or `sito.exe` is not validly signed (`Get-AuthenticodeSignature` status `Valid`) and that verifies the publisher name.
- **Smoke test** (`.github/workflows/release.yml`) keeps passing with the signed installer.
- **Updater:** the in-app update ([slice 41](41-update-check.md)) runs the downloaded installer. Check that a signed installer installs over an unsigned one and that its minisign check still matches.
- **Antivirus:** slice 2 noted that G DATA might flag the unsigned installer; signing should reduce such false positives.
- **Who tests it:** the user has no Windows machine (slice 41). Someone downloads the installer in Edge on Windows and starts it, or a CI job checks the signature at least.
- **Docs:** `docs/RELEASING.md` (signing setup, how to install without the SmartScreen steps), the macOS-only signing note in the release workflow, CLAUDE.md "CI" ("Unsigned for now"), and a `docs/domain/` note if users see something new (publisher name).

## Related slices

- **macOS signing and notarization** ([slice 23](23-macos-polish.md)) is the same kind of work for the other platform: the same release workflow step pattern (secrets, a check before publishing, docs). Build the verification step so macOS can add its own check (`codesign --verify`, `spctl`) next to it.
- **Install anywhere** ([slice 35](35-install-anywhere.md)): a per-machine install shows a UAC prompt; with a signed installer it names the publisher instead of "Unknown publisher".
- **Zero-click update** ([slice 42](42-zero-click-update.md)): a silent update install is more trustworthy with a signed installer, and on locked-down machines it may need one.

## Try

Download the installer of a signed release in Edge on Windows: there is no SmartScreen block, or at least the warning names the publisher. Right-click the installer → Properties → Digital Signatures shows the publisher and a timestamp. After installing, `sito.exe` is signed too, and an update from the previous release installs as before.
