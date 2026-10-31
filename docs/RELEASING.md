# Releasing Sito

Installers are built by GitHub Actions (`.github/workflows/release.yml`), never locally. A release build is too heavy for the low-memory dev machine.

## Test build (no release)

Run the Release workflow manually. It builds the installers, smoke-tests the Windows installer (install, launch, open the viewer, uninstall), and attaches them to the workflow run as artifacts.

```bash
gh workflow run release.yml --ref main
gh run list --workflow Release --limit 1
gh run download <run-id> --name sito-windows-latest --dir D:\Lukasz\tools\installers
```

## Release

1. Add a `## vX.Y.Z – YYYY-MM-DD` section at the top of `CHANGELOG.md`: what changed since the last release tag (`git log vPREV..HEAD`), in English, from the user's point of view and grouped as New / Improved / Fixed (see CLAUDE.md for how to word it). Check it with `node scripts/release-notes.mjs vX.Y.Z`. The release workflow publishes this section, followed by the installation notes, as the GitHub release notes, and fails early for a tag without one.
2. Bump `version` in `[workspace.package]` of the root `Cargo.toml` (it is the only place the version lives; `tauri.conf.json` takes it from there). Use [semantic versioning](https://semver.org).
3. Run `cargo check` so `Cargo.lock` picks up the new version, then commit both: `chore: release vX.Y.Z`.
4. Tag and push:

   ```bash
   git tag vX.Y.Z
   git push origin main vX.Y.Z
   ```

5. The Release workflow checks that the tag matches `Cargo.toml` and has a changelog section, builds Windows and macOS, installs, launches and uninstalls the Windows app, checks the update manifest (`latest.json`), then publishes the GitHub release **Sito vX.Y.Z** with the changelog as its notes. If any step fails, the release stays a draft.

Publishing is what ships the update: installed apps read `latest.json` of the newest published release on their next start (see [Updates](domain/updates.md)). A draft is never offered.

## Update signing key

Updates are signed with a minisign key pair, separate from code signing. The public key is in `plugins.updater.pubkey` of `src-tauri/tauri.conf.json` and built into every app; the app installs only updates signed with the matching private key.

**One-time setup** (on the dev machine, never in a cloud session, so the private key stays there):

```bash
npx tauri signer generate -w D:\Lukasz\tools\keys\sito-updater.key   # asks for a password
gh secret set TAURI_SIGNING_PRIVATE_KEY < D:\Lukasz\tools\keys\sito-updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD                       # the same password
```

Put the contents of `sito-updater.key.pub` into `plugins.updater.pubkey` (replacing the placeholder, which makes the Release workflow fail) and commit it.

**Back up the private key and its password outside GitHub** (e.g. in a password manager): secrets cannot be read back. If the key is lost, installed apps can no longer update: generate a new pair, release with the new public key, and every user installs that release by hand once.

**Update manifest:** with `bundle.createUpdaterArtifacts`, the build writes a `.sig` next to each installer and `tauri-action` uploads `latest.json` (version, the tag's CHANGELOG changes as notes, a signed download URL per platform) to the release. The publish job refuses a manifest without the tag's version, notes, or a signed Windows and macOS entry (`scripts/check-update-manifest.mjs`).

## Testing an update locally

1. Run the Release workflow manually and download the Windows artifact: the installer and its `.sig`.
2. Install it, then write a manifest that offers it as a newer version and serve its folder:

   ```bash
   node scripts/local-update-manifest.mjs D:\Lukasz\tools\installers\Sito_X.Y.Z_x64-setup.exe 99.0.0
   cd D:\Lukasz\tools\installers && python -m http.server 8000
   ```

3. Start a debug build against it: `$env:SITO_UPDATE_ENDPOINT = 'http://localhost:8000/latest.json'; npm run tauri dev`. Only debug builds read this variable, and only they accept `http://`. The notice offers 99.0.0; installing runs the downloaded installer, which restarts the installed Sito.

## Third-party notices

Every build contains `THIRD-PARTY-NOTICES.txt` (shown under „Pomoc → Licencje…”): Sito's license, the Rust crates from the committed `ui/notices/rust-crates.txt`, and the npm packages of the bundle. CI keeps the Rust part up to date and fails on unreviewed licenses, so a green `main` needs no extra release step.

## Installing on Windows

1. Download `Sito_X.Y.Z_x64-setup.exe` from the [releases page](https://github.com/lukaszkalnik/sito/releases), or with `gh release download vX.Y.Z --pattern "*setup.exe"`.
2. Run it. No admin rights are needed: Sito installs for the current user only (`%LOCALAPPDATA%\Sito`), adds a Start menu entry and, only if you tick it on the last page (unticked by default), a desktop shortcut.
3. The installer is not code-signed, so Windows SmartScreen shows "Windows protected your PC" ("System Windows ochronił ten komputer"). Click **More info** ("Więcej informacji"), then **Run anyway** ("Uruchom mimo to").

**Updating:** from the first release with the updater on, Sito offers a newer version itself on start („Zainstaluj i uruchom ponownie”). Older versions update by running the newer installer by hand; it replaces the old version in place, with "Nie odinstalowuj" (keep the installed version and install over it) preselected.

**Installer template:** `src-tauri/installer/installer.nsi` is a copy of Tauri's NSIS template with two changes marked `Sito:` (those two defaults). After updating `@tauri-apps/cli`, re-copy the upstream template for the new version and re-apply them.

**Uninstalling:** Settings → Apps → Installed apps → Sito → Uninstall.

## Regenerating the app icon

The icon source is `src-tauri/icons/source/icon.svg`. After editing it:

```bash
node scripts/render-icon.mjs                      # SVG → 1024×1024 PNG (Playwright Chromium)
npx tauri icon src-tauri/icons/source/icon.png    # all sizes in src-tauri/icons
```

Delete the generated `src-tauri/icons/android` and `src-tauri/icons/ios` folders (desktop only), and copy the SVG to `ui/public/favicon.svg`.
