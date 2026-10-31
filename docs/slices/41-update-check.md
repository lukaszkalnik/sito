# Slice 41 – Update check on start

**Status:** 📝 concept drafted (2026-10-31; plan in plan mode when the slice starts)

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
