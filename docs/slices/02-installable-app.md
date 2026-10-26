# Slice 2 – Installable app

**Status:** 🚧 in progress

## Context

Slice 1 works, but only through `npm run tauri dev`, which needs the whole dev setup and a long compile on a low-memory PC. The user wants Sito to run like a regular desktop app: install once, then start it with one click. From this slice on, every later slice can be tested in the installed app.

User decisions:
- **Manual updates for now:** a new version is installed by downloading the new installer and running it, which upgrades in place. A built-in updater can become its own slice later. It needs an update-signing key, and can serve updates straight from the public GitHub releases.
- **Icon:** Claude draws a simple flat sieve icon (round sieve with a mesh pattern, one accent color). Easy to replace later.

Constraints found:
- `src-tauri/tauri.conf.json` currently bundles `msi` (per-machine, needs admin rights), `dmg` and `app`, with Tauri's default icons, and repeats the version (`0.1.0`) that the Cargo workspace already defines.
- `.github/workflows/release.yml` builds on `v*` tags into a draft release. It has no smoke test, no manual trigger and no version check.
- The installer is unsigned, so Windows SmartScreen will warn on first run ("Windows protected your PC" → *More info* → *Run anyway*). Code signing costs money and is out of scope; documented instead.

## Implementation

### 0. Docs
- Save this plan as `docs/slices/02-installable-app.md`; ROADMAP slice 2 → 🚧 with a link to the plan.
- Commit `docs: add slice 2 plan`.

### 1. App icon
- `src-tauri/icons/source/icon.svg`: the sieve icon drawn as SVG (rounded square background, sieve rim and handle, mesh grid). The source is committed so it can be edited later.
- Render it to a 1024×1024 PNG with the Playwright Chromium that's already installed (small script `scripts/render-icon.mjs`, run once and documented), then `npx tauri icon src-tauri/icons/source/icon.png` regenerates every size (`.ico`, `.icns`, PNGs) in `src-tauri/icons/`.
- Use the same icon as the UI favicon (`ui/public/favicon.svg`).
- Commit `feat: add Sito app icon`.

### 2. Bundle configuration (`src-tauri/tauri.conf.json`)
- `bundle.targets`: `["nsis", "dmg", "app"]`. NSIS replaces MSI because it installs per-user without admin rights and upgrades in place.
- `bundle.windows.nsis`: `installMode: "currentUser"`, `languages: ["Polish"]`, `displayLanguageSelector: false`. The installer creates a Start menu entry, and its last page has a checkbox for a desktop shortcut (Tauri's NSIS template; verify on the built installer).
- Metadata shown in Windows "Installed apps": `publisher: "Lukasz Kalnik"`, `category: "Photography"`, Polish `shortDescription`/`longDescription`, `copyright`.
- Remove `version` from `tauri.conf.json`, so Tauri takes it from the Cargo package (workspace `version`, `0.1.0`). That leaves one place to bump.
- Release builds already hide the console window (`src-tauri/src/main.rs`).

### 3. Release workflow (`.github/workflows/release.yml`)
- Triggers: `v*` tags (real release) and `workflow_dispatch` (test build: same steps, artifacts uploaded to the run instead of a release).
- **Version check:** on tags, fail early if the tag (e.g. `v0.1.0`) differs from the workspace version in `Cargo.toml`.
- Windows and macOS build jobs via `tauri-action` into a **draft** release (as today).
- **Windows smoke test**, same job, after the build: run the NSIS installer silently (`/S`), assert `sito.exe` exists in the per-user install folder (`%LOCALAPPDATA%\Sito`), start it, check it's still running after 10 s, stop it, run the uninstaller silently, assert the folder is gone.
- **Publish job** (needs both builds): turns the draft into a published release (`gh release edit --draft=false`), so a release only appears once everything succeeded. The repo is public, so published releases and their installers are downloadable by anyone.
- Commit `ci: build per-user Windows installer with smoke test`.

### 4. Release docs
- `docs/RELEASING.md`: bump `version` in the root `Cargo.toml`, commit, `git tag vX.Y.Z`, `git push --tags`, wait for the Release workflow, download `Sito_X.Y.Z_x64-setup.exe` from the GitHub release. Includes the SmartScreen note and how to uninstall (Settings → Apps).
- `CLAUDE.md`: CI section says NSIS instead of MSI, and links `docs/RELEASING.md` from Commands; how to regenerate icons.
- Commit `docs: describe the release process`.

### 5. First release
- Run the workflow manually first (`gh workflow run release.yml`), check the smoke test and artifacts.
- Then tag `v0.1.0` and push it. The workflow builds, smoke-tests and publishes **Sito v0.1.0**.
- ROADMAP slice 2 → 🧪 (ready for testing), and ✅ once you've tested it.

## Verification
1. Local: `cargo fmt/clippy/test`, `npm run lint`, `npm test`, `npm run e2e` stay green. Config-only changes, plus `tauri.conf.json` validated by the Tauri CLI schema. No local release build, because it's too heavy for this PC; CI does it.
2. CI: the manual `workflow_dispatch` run and then the `v0.1.0` tag run are green on Windows and macOS, including the install/launch/uninstall smoke test.
3. Manual (you):
   - Download `Sito_0.1.0_x64-setup.exe` from the release, run it (SmartScreen: *More info* → *Run anyway*). The installer is in Polish and needs no admin prompt.
   - Start Sito from the Start menu and from the desktop shortcut. The window shows the sieve icon, the taskbar shows the right icon and title, and the slice 1 flow (pick folder → count) works.
   - Settings → Apps lists "Sito" with publisher and version, and uninstalling works.

Risks / notes:
- G DATA might flag the unsigned installer. If so, allow it once (or report it as a false positive).
- macOS build stays unsigned until slice 14; the DMG is built but not tested now.
