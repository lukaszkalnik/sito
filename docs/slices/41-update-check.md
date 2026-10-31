# Slice 41 – Update check on start

**Status:** 📝 planned (2026-10-31; plan approved, implementation not started)

## Idea

Asked by the user on 2026-10-31: Sito looks for a newer version every time it starts and, when there is one, offers it in the app. Installing it is **one click**: Sito downloads the new installer, installs it and restarts, instead of the user finding the release on GitHub and running the installer by hand ([Releasing](../RELEASING.md#installing-on-windows)).

This is stage 1 of auto update; stage 2, installing without any click, is [slice 42](42-zero-click-update.md). Stage 1 carries all the infrastructure (signing, update manifest, check), so stage 2 only changes *when* the update is applied.

**Bootstrapping:** only installs that already contain the updater can update themselves. The first release with this slice has to be installed by hand once more; from then on updates arrive in the app. So this slice should be released soon after it is done.

## Feasibility

Yes, with Tauri's official updater plugin (`tauri-plugin-updater`, MIT/Apache-2.0) and the GitHub releases we already publish:

- **Signed updates.** Tauri signs every update bundle with a minisign key pair; the public key is built into the app, and the app refuses an update whose signature does not match. This is independent of code signing (the installer stays unsigned for now). The private key and its password go into the repository secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; **losing the key means installed apps can no longer update** (the user would install by hand once), so it also needs a backup outside GitHub.
- **Update manifest.** With `bundle.createUpdaterArtifacts: true`, `tauri-action` builds the `.sig` files and uploads a `latest.json` (version, notes, per-platform URL and signature) to the release. The app checks `https://github.com/lukaszkalnik/sito/releases/latest/download/latest.json`; `latest` only points at published releases, so a draft that failed its smoke test is never offered.
- **Windows install.** The updater downloads the NSIS installer and runs it (`installMode` `passive`: a small progress window, no questions). Per-user installs need no admin rights, so there is no UAC prompt. The download does not get the browser's „mark of the web”, so SmartScreen should not ask either (to verify).
- **macOS** gets an updater bundle (`.app.tar.gz`) as well, but an unsigned, unnotarized app replacing itself is untested; it belongs with signing in [slice 23](23-macos-polish.md). Until then macOS may only show the notice with a link to the release page.

## Candidate UI

- At start, after the main window is shown, the check runs in the background. It never delays the start and never blocks the grid.
- **No update / offline / GitHub unreachable:** nothing is shown (the error goes to the debug log). A failed check is not an error the user has to act on.
- **Update found:** a non-modal notice in the main window (e.g. the bottom note area of [slice 37](37-steady-top-bar.md), or the top bar): „Dostępna jest nowa wersja 0.15.0” with „Zainstaluj i uruchom ponownie” and „Co nowego?” (opens the release notes; `latest.json` carries the changelog section) and a way to dismiss it for this session.
- **Installing:** download progress („Pobieranie aktualizacji… 45%”), then the installer runs and Sito restarts. Not offered while a reject/accept or a scan write is in progress; the app closes the same way as when the user closes it (window state saved, background work stopped).
- **„Pomoc → Sprawdź aktualizacje…”** checks on demand and also says „Masz najnowszą wersję (0.14.0)” or why the check failed.

## Hard rules this touches

- **Network access.** CLAUDE.md allows network access only for the model download; the update check becomes the second stated exception („Security and privacy”), and the CSP / capabilities get only what the updater needs. The request carries no photo data: it is a plain GET of `latest.json` from GitHub (GitHub sees the IP address, like any download from the releases page). The domain docs say this plainly.
- **The app only suggests:** for photos. Updating the app itself is offered, not forced, in this slice.

## To check when planning

- Plugin setup: Rust crate + `@tauri-apps/plugin-updater` (and `plugin-process` for the restart) or do it all on the Rust side, keeping the UI behind typed wrappers in `ui/src/features/update/api.ts`. Rust side preferred, so the check can start without the UI and errors map to codes.
- Release workflow: secrets, `createUpdaterArtifacts`, check that `latest.json` lists both platforms after the matrix builds (`tauri-action` merges them), and that the publish job (draft → published) keeps the assets.
- **Testing the update path** end to end: a debug-only override of the endpoint (env var, like `SITO_SMOKE_TEST`) pointing at a local `latest.json`, so the smoke test or a manual run can update from the just-built installer to itself (or to a version-bumped build) without a real release. Unit tests for the version comparison and the notice state machine (`idle → checking → available → downloading → installing`, `failed`).
- Where the notice lives so it does not make the bar jump ([slice 37](37-steady-top-bar.md)).
- [Slice 35](35-install-anywhere.md): a per-machine install needs admin to update; the updater's installer run must still work (UAC prompt) or the notice must say so.
- `docs/RELEASING.md` (signing key, its backup, the manifest), `docs/domain/` (a new topic file, e.g. `updates.md`), CHANGELOG wording for the first release with the updater.

## Try

Install the previous release, publish (or simulate) a newer one, start Sito: the notice appears within a few seconds; „Zainstaluj i uruchom ponownie” updates and restarts without SmartScreen or UAC. Start offline: no notice, no error. „Pomoc → Sprawdź aktualizacje…” with no newer version says so.

## Plan (approved 2026-10-31)

### Context

Today a new Sito version reaches the user only by downloading the installer from GitHub and running it by hand. Slice 41 (concept in `docs/slices/41-update-check.md`) makes Sito check for a newer release on every start and install it with one click (download, signed-update check, NSIS installer in passive mode, restart). It also builds the base for slice 42 (zero-click): signing, the update manifest, the check, the state machine of the notice.

Findings that shape the plan:
- `tauri-plugin-updater` 2.14.1 (MIT/Apache-2.0) does everything from Rust: `check()`, `download(on_chunk, ..)`, `install(bytes)`. On Windows `install` runs the NSIS installer with `/P /UPDATE /R` (passive, restart) and then `std::process::exit(0)`; `UpdaterBuilder::on_before_exit` runs a hook just before. `endpoints()` accepts `http://` only in debug builds, so a local test manifest works in `npm run tauri dev` and can never be used by a release build.
- New Rust license brought in: **ISC** (`ring`, `rustls-webpki`, `untrusted`; `rustls` is also ISC-or-MIT). Permissive (MIT-like), to be added to `about.toml`. Everything else is already accepted.
- `tauri-plugin-process` is **not** needed: the installer restarts the app (`/R`).
- Thumbnails are written atomically (`crates/core/src/thumbnail.rs`, temp + rename); the action log writes its undo entry before the move, so an exit is safe like a crash. Still, the exit waits for a running reject/accept.
- tauri-action writes `latest.json` with `notes` = `releaseBody`, which today is a placeholder ("Release notes follow…"); the CHANGELOG section must be passed at build time instead.

### Decisions

- **All updater calls on the Rust side** (`src-tauri/src/update.rs`). The webview gets no `updater:*` permission, and the CSP stays unchanged: HTTP goes out from Rust, not from the webview. The only new network access is a GET of `https://github.com/lukaszkalnik/sito/releases/latest/download/latest.json` and, after a click, the installer download. CLAUDE.md „Security and privacy” names it as the second exception.
- **The check runs when the main window's UI mounts** (it calls `check_for_update`). That is right after the window is shown, never delays the start, and no event can be missed. It always runs in the background (async command).
- **The notice is its own floating card at the bottom right of the main window** (`position: fixed`). It does not touch the bar (slice 37) or the background-work pill at the bottom centre, and it shows over the start screen as well as over the grid. The viewer window gets nothing.
- **macOS:** the check runs and the notice shows the version and „Co nowego?”, but instead of the install button it says to download the new version from the releases page (`install: "manual"`). Installing in the app on macOS comes with signing (slice 23). There is no clickable link yet, because it would need the opener plugin.
- **„Co nowego?”** opens a dialog with the release notes from `latest.json`: the English CHANGELOG section, shown as plain text like the licenses dialog.
- **Dismiss:** „Nie teraz” hides the notice until the next start (state in the hook, no storage).

### Steps

#### 1. One-time setup by the user (blocking for a real release, not for development)
- On the dev machine: `npx tauri signer generate -w D:\Lukasz\tools\keys\sito-updater.key` (with a password). Back up the key and password outside GitHub (e.g. a password manager).
- `gh secret set TAURI_SIGNING_PRIVATE_KEY < ...key` and `gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Give me the public key (`.key.pub`), which goes into `tauri.conf.json`. I do not generate the key in the cloud container, so the private key never leaves the user's machine.

#### 2. Rust: `src-tauri/src/update.rs` (+ `update/` submodules if it grows)
- Cargo: `tauri-plugin-updater = { version = "2.14", default-features = false, features = ["rustls-tls", "system-proxy"] }` (no `zip`: only needed to install the macOS `.app.tar.gz`). Register `tauri_plugin_updater::Builder::new().build()` in `lib.rs`.
- `tauri.conf.json`: `bundle.createUpdaterArtifacts: true`; `plugins.updater: { pubkey, endpoints: [<latest.json URL>], windows: { installMode: "passive" } }`.
- `UpdateState(Mutex<Option<tauri_plugin_updater::Update>>)` in `tauri::State`: the update found by the last check, so the install installs exactly what the user was shown.
- Commands (thin):
  - `check_for_update() -> Result<Option<UpdateInfo>, UpdateError>`. `UpdateInfo { version, notes: Option<String>, install: InstallKind }`, with `InstallKind::{InApp, Manual}` (`Manual` on macOS via `cfg`).
  - `install_update(on_progress: Channel<DownloadProgress>) -> Result<(), UpdateError>`. Downloads and reports `{ downloaded, total: Option<u64> }` through the existing `scan::completion` `Throttle` pattern. That pattern gets extracted into a shared `src-tauri/src/throttle.rs` (in a separate `refactor:` commit, since it is now used in a second place) and sends at most every 100 ms plus once at the end. Then it installs. On Windows the process ends there; `Manual` returns `UpdateError::Unsupported`.
  - `on_before_exit` hook: `app.save_window_state(window_fit::STATE_FLAGS)` (as `window_fit::save_on_close`) and take the `reject::Actions` lock so that a running reject/accept finishes first. The guard is kept until the exit, with a comment explaining why.
- Debug-only endpoint override: `SITO_UPDATE_ENDPOINT` (read only under `cfg!(debug_assertions)`, like `smoke::ENV`), set through `app.updater_builder().endpoints(..)`.
- `UpdateError` (serializable code enum, like `error::CommandError`): `Network` (offline, GitHub unreachable, bad manifest), `BadSignature`, `NoUpdate` (the install was called without a pending update), `Unsupported`, `Internal`. It comes from a pure `fn from_plugin(&tauri_plugin_updater::Error) -> UpdateError`, with unit tests. Errors are logged (`log::warn!`).
- Menu (`about.rs`): new item „Sprawdź aktualizacje…” in Pomoc (on macOS in the Help menu), `MenuAction::CheckForUpdates` → event `update-check-requested` to `main`. Tests extended in `about.rs`.

#### 3. UI: `ui/src/features/update/`
- `api.ts`: typed wrappers `checkForUpdate()`, `installUpdate(onProgress)`, `onCheckRequested(handler)`, plus validation of the response.
- `updateReducer.ts` (pure, Vitest): state `idle | checking{manual} | available{info} | upToDate{version} | failed{code, manual} | downloading{info, percent|null} | installing | dismissed`. Events: `checkStarted(manual)`, `found`, `notFound`, `checkFailed`, `installStarted`, `progress`, `installFailed`, `dismissed`. Rules: the automatic check shows nothing for `notFound`/`failed` (the error goes to `console.error`); a manual check shows „Masz najnowszą wersję (0.14.0)” or the failure; an install failure returns to `available` with the error message. Exhaustive `switch` + `assertNever`.
- `useUpdate.ts`: check on mount, listener for the menu event (cleaned up), install with the progress channel.
- `UpdateNotice.tsx` + `.css`: a card at the bottom right, `aria-live="polite"`, native buttons „Zainstaluj i uruchom ponownie”, „Co nowego?”, „Nie teraz” (or the text for `Manual`). Progress „Pobieranie aktualizacji… 45%” (`Intl.NumberFormat('pl-PL', {style:'percent'})`), then „Instalowanie…”. Buttons disabled while installing.
- `ReleaseNotesDialog.tsx`: modeled on `about/LicensesDialog.tsx` (the shared dialog shell gets extracted only if it matches 1:1, otherwise kept separate).
- Mounted in `ui/src/App.tsx` next to `LicensesDialog`.
- `pl.json`: an `update.*` section (texts above + error codes `update.errors.<code>`).
- e2e: `ui/e2e/update.spec.ts` with mocks in `tauri-mock.ts` (update found → notice → „Co nowego?” → install with progress; no update → nothing; menu event with no update → „Masz najnowszą wersję”). The default mock returns `null`, so existing tests do not change.

#### 4. Release workflow (`.github/workflows/release.yml`) and scripts
- tauri-action step: `env` `TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)`; `releaseBody` = the changes-only CHANGELOG section (`node scripts/release-notes.mjs "$TAG" --changes-only`, new flag), so `latest.json` carries real notes. The publish job keeps setting the full notes (with installation notes) on the release page.
- Manual runs: the artifact also carries `*-setup.exe.sig`.
- Publish job: before un-drafting, `scripts/check-update-manifest.mjs` downloads `latest.json` from the draft (`gh release download`) and fails if `windows-x86_64` (and the darwin entries) has no `url`/`signature`. That catches a lost entry when the matrix jobs merge it. The release then stays a draft, so `latest` is never broken.
- `scripts/local-update-manifest.mjs <setup.exe> <version>`: writes a `latest.json` for a local test (signature from the `.sig`, URL `http://localhost:8000/...`).

#### 5. Docs
- `docs/domain/updates.md` (new, + index in `docs/domain/README.md`): when the check runs, what is sent (a GET, IP visible to GitHub, no photo data), the notice and its states, macOS, behaviour offline.
- CLAUDE.md „Security and privacy”: second network exception (update check/download, signature-verified).
- `docs/RELEASING.md`: signing key, its backup, what happens when it is lost, `latest.json`, local update test; under „Updating” the note that updates arrive in the app from version X.
- `about.toml` + `docs/domain/licenses.md`: ISC on the accepted list; `npm run notices:rust`, commit `ui/notices/rust-crates.txt`.
- Slice 35: note that a per-machine install will need a UAC prompt for the update; slice 42: what it builds on (the reducer, `UpdateState`, `install_update`).
- Slice 41 file: save this plan, status 📝→🚧→🧪 in sync with the roadmap.

#### Commits (each one with green checks, pushed to `main`)
1. `docs:` plan saved in the slice file. **Pause:** a chance to clear the context.
2. `refactor:` shared `Throttle`.
3. `feat:` Rust updater + commands + menu + license/notices (+ pubkey once the user provides it; until then a placeholder makes the build fail with `createUpdaterArtifacts`, so this commit waits for the key).
4. `feat:` UI notice + reducer + e2e.
5. `ci:` workflow + scripts.
6. `docs:` domain/RELEASING/CLAUDE.md, status 🧪. **Pause.**

### Verification
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (tests for `from_plugin`, `MenuAction`, `Throttle`).
- `npm run lint`, `npm run typecheck -w ui`, `npm test` (reducer, `UpdateNotice`), `npm run e2e` (new `update.spec.ts` + existing).
- CI manual run of the Release workflow: build with signatures, `.sig` in the artifact, smoke test green.
- Manual test by the user (Windows): install the artifact; `node scripts/local-update-manifest.mjs Sito_…-setup.exe 99.0.0`, `npx http-server`/`python -m http.server 8000` in the artifact's folder, `SITO_UPDATE_ENDPOINT=http://localhost:8000/latest.json npm run tauri dev`: the notice shows 99.0.0 → „Co nowego?” → install → passive installer without UAC/SmartScreen → Sito restarts (installed version). Offline: no notice. „Pomoc → Sprawdź aktualizacje…” without an update: „Masz najnowszą wersję”.
- Real test: the first release with the updater gets installed by hand; the next release shows up in the app.
