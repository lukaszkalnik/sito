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

## Pinned actions

The Release workflow holds the update signing key and publishes what every installed Sito offers as an update, so nothing in it may change unnoticed:

- Every action in `.github/workflows/` is pinned to a full commit SHA, with its version in a comment (`# v7.0.1`). A tag can be moved by the action's owner (or by whoever takes the repository over); a commit cannot. `dtolnay/rust-toolchain` has no releases: it is pinned to a commit of its `master` branch and installs `toolchain: stable`.
- Every workflow starts with `permissions: contents: read`; only the two release jobs get `contents: write`. Checkouts do not keep the token (`persist-credentials: false`).
- The release build restores no cache and builds with `--locked`, so it is made from the sources and the committed lockfiles alone.
- `zizmor` (the `workflows` job of CI) fails on a workflow that breaks these rules. Run it locally with `zizmor .github` (`brew install zizmor`). A finding that does not apply is silenced in `.github/zizmor.yml` with the reason.

**Updating the pins:** Dependabot (`.github/dependabot.yml`) opens one pull request a month with all action updates, changing each SHA and its version comment together. These pull requests are the one exception to working on `main` only. Before merging one:

1. Read the release notes of each updated action for changes that affect the workflows.
2. Check that each new SHA is the commit of the tagged release: `gh api repos/<owner>/<action>/git/ref/tags/<tag>` (for an annotated tag, follow it with `gh api repos/<owner>/<action>/git/tags/<sha>`).
3. Wait for CI to be green, merge, then run the Release workflow by hand (see [Test build](#test-build-no-release)) to see that the installer still builds.

**`tauri-action` stays on v0** (Dependabot ignores its major updates): v1 writes `api.github.com` asset addresses into `latest.json` instead of the release download addresses, which changes where installed apps fetch an update from. Moving to v1 is [slice 58](slices/58-tauri-action-v1.md), together with a tested update from a published release.

## Security checks

- **Audit workflow** (`.github/workflows/audit.yml`): `cargo audit` and `npm audit --omit=dev` on every push to `main` and every Monday. It fails on a known vulnerability in a shipped dependency; GitHub emails the repository owner when a scheduled run fails. Do not release while it is red.
- **CodeQL** (GitHub's default setup, a repository setting with no workflow file): scans the Rust, JavaScript/TypeScript and workflow code on pushes and weekly. Results are under Security → Code scanning; fix a real finding, dismiss another one there with a reason.
- **Secret scanning with push protection** ("Secret Protection" under Settings → Code security): GitHub refuses a push that contains a key or token, which guards the update signing key above all. Alerts are under Security → Secret scanning.

## Update signing key

Updates are signed with a minisign key pair, separate from code signing. The public key is in `plugins.updater.pubkey` of `src-tauri/tauri.conf.json` and built into every app; the app installs only updates signed with the matching private key.

**Current key:** minisign key ID `16FBDC84716AF028`, generated on 2026-11-05 on the Mac. The private key and its password are kept in the user's 1Password (the only copy outside the GitHub secrets) and in the repository secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Releases are built by CI, so no machine needs the key for a normal release.

**Setting up a new key** (only if the key is lost or must be replaced; run by the user on their own machine, never in a cloud session or by Claude, so the private key and password stay with the user):

```bash
mkdir -p ~/keys && npx tauri signer generate -w ~/keys/sito-updater.key   # asks for a password
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/keys/sito-updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD                            # prompts for the same password
```

Put the contents of `sito-updater.key.pub` into `plugins.updater.pubkey` (the Release workflow fails while it is the placeholder) and commit it.

**Back up the private key and its password outside GitHub** (in 1Password): secrets cannot be read back. If the key is lost, installed apps can no longer update: generate a new pair, release with the new public key, and every user installs that release by hand once.

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

## Installing on macOS

1. Download `Sito_X.Y.Z_universal.dmg` (runs on Apple Silicon and Intel) from the [releases page](https://github.com/lukaszkalnik/sito/releases), or with `gh release download vX.Y.Z --repo lukaszkalnik/sito --pattern "*.dmg" --dir ~/Downloads`.
2. Open it and drag **Sito** into **Applications**.
3. The app is not signed yet ([slice 50](slices/50-macos-code-signing.md)), so macOS refuses the first start ("Sito" Not Opened). Click **Done**, then open **System Settings → Privacy & Security**, scroll down to the **Security** section, where it says Sito was blocked, and click **Open Anyway**; confirm with your password and **Open Anyway** again. Right-click → **Open** no longer works for this since macOS 15. If macOS reports the app as damaged instead, run `xattr -dr com.apple.quarantine /Applications/Sito.app` once.

**Updating:** Sito shows a notice when a newer version is out; install it by downloading the new DMG and replacing the app (installing from the app comes with signing, see [Updates](domain/updates.md#installing)).

## Regenerating the app icon

The icon source is `src-tauri/icons/source/icon.svg`. After editing it:

```bash
node scripts/render-icon.mjs                      # SVG → 1024×1024 PNG (Playwright Chromium)
npx tauri icon src-tauri/icons/source/icon.png    # all sizes in src-tauri/icons
```

Delete the generated `src-tauri/icons/android` and `src-tauri/icons/ios` folders (desktop only), and copy the SVG to `ui/public/favicon.svg`.
