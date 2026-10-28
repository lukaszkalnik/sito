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

1. Bump `version` in `[workspace.package]` of the root `Cargo.toml` (it is the only place the version lives; `tauri.conf.json` takes it from there). Use [semantic versioning](https://semver.org).
2. Run `cargo check` so `Cargo.lock` picks up the new version, then commit: `chore: release vX.Y.Z`.
3. Tag and push:

   ```bash
   git tag vX.Y.Z
   git push origin main vX.Y.Z
   ```

4. The Release workflow checks that the tag matches `Cargo.toml`, builds Windows and macOS, installs, launches and uninstalls the Windows app, then publishes the GitHub release **Sito vX.Y.Z**. If any step fails, the release stays a draft.

## Installing on Windows

1. Download `Sito_X.Y.Z_x64-setup.exe` from the [releases page](https://github.com/lukaszkalnik/sito/releases), or with `gh release download vX.Y.Z --pattern "*setup.exe"`.
2. Run it. No admin rights are needed: Sito installs for the current user only (`%LOCALAPPDATA%\Sito`), adds a Start menu entry and, only if you tick it on the last page (unticked by default), a desktop shortcut.
3. The installer is not code-signed, so Windows SmartScreen shows "Windows protected your PC" ("System Windows ochronił ten komputer"). Click **More info** ("Więcej informacji"), then **Run anyway** ("Uruchom mimo to").

**Updating:** run the newer installer; it replaces the old version in place. "Nie odinstalowuj" (keep the installed version and install over it) is preselected. There is no built-in updater yet.

**Installer template:** `src-tauri/installer/installer.nsi` is a copy of Tauri's NSIS template with two changes marked `Sito:` (those two defaults). After updating `@tauri-apps/cli`, re-copy the upstream template for the new version and re-apply them.

**Uninstalling:** Settings → Apps → Installed apps → Sito → Uninstall.

## Regenerating the app icon

The icon source is `src-tauri/icons/source/icon.svg`. After editing it:

```bash
node scripts/render-icon.mjs                      # SVG → 1024×1024 PNG (Playwright Chromium)
npx tauri icon src-tauri/icons/source/icon.png    # all sizes in src-tauri/icons
```

Delete the generated `src-tauri/icons/android` and `src-tauri/icons/ios` folders (desktop only), and copy the SVG to `ui/public/favicon.svg`.
