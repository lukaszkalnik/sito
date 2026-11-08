---
status: ✅ done
---

# Slice 41a – Link to the release page in the update notice

Since 2026-11-08; plan approved on 2026-11-08, see [Plan](#plan). Implemented on 2026-11-08, see [Implementation notes](#implementation-notes); what it established is in [Updates](../domain/updates.md). Tested by the user on the Mac on 2026-11-08 with a debug build and a local `latest.json` announcing 0.16.0: „Pobierz” opened `…/releases/tag/v0.16.0` in the browser (a 404, as expected, since that test version has no release).

## Idea

Found by the user on 2026-11-08 while testing [slice 41](41-update-check.md) on the Mac: the notice says „Dostępna jest nowa wersja Sito: 0.15.1. Pobierz ją ze strony wydań Sito na GitHubie.” but gives no way to get there; the user has to find the page themselves. Slice 41 left the link out on purpose, since opening the browser needs a new capability ([Updates](../domain/updates.md), „macOS”).

The macOS notice gets a button (e.g. „Pobierz”) that opens the release page of the offered version (`https://github.com/lukaszkalnik/sito/releases/tag/vX.Y.Z`) in the system browser. It is the macOS counterpart of „Zainstaluj i uruchom ponownie” and stays until in-app install comes with signing ([slice 50](50-macos-code-signing.md)).

## Candidate approaches

- **A Rust command that builds the URL itself (preferred).** `open_release_page(version)` validates the version, builds the GitHub URL from a constant and opens it with the system opener (`tauri-plugin-opener`'s Rust API or the `open` crate). The UI never passes a URL, so no capability lets the webview open arbitrary addresses.
- **The opener plugin's JS API**, scoped in the capability to `https://github.com/lukaszkalnik/sito/releases/*`. Less Rust, but a webview permission.

Either way, CLAUDE.md asks for a stated reason for the new permission, and the dependency's license goes through the usual review (`about.toml`, `npm run notices:rust`).

## Related

- [Slice 54](54-release-notes-markdown.md) (formatted release notes) may want clickable links in the notes; build the opener so it can take a checked GitHub URL later, not only the release page.
- [Slice 50](50-macos-code-signing.md): with signing, macOS installs in the app and this button goes away.

## Plan

Approved on 2026-11-08.

### Approach: a Rust command with no input

`open_release_page` takes **no arguments**. It reads the version of the update found by the last check from `UpdateState` (`src-tauri/src/update.rs`), the same update `install_update` installs, builds `https://github.com/lukaszkalnik/sito/releases/tag/v{version}` from a constant and opens it with **`tauri-plugin-opener`'s Rust API** (`app.opener().open_url(..)`). The UI never passes a URL and the capability file gets **no** opener permission, so the webview cannot open anything itself (the stated reason for adding the plugin: Rust-only use). The version was already parsed as semver by the updater plugin; the URL is still built with `Url::path_segments_mut`, so nothing in it can leave the path.

For [slice 54](54-release-notes-markdown.md) (links in the notes), the URL check is a separate pure function `github_page(url) -> bool` that accepts only `https://github.com/lukaszkalnik/sito/…`; the release page goes through it too, so slice 54 can add an `open_release_link(url)` command on top.

### Rust (`src-tauri`)

- `Cargo.toml`: add `tauri-plugin-opener = "2"` (MIT/Apache-2.0; check `about.toml`). Register the plugin in `lib.rs`.
- New `src-tauri/src/update/release_page.rs`: `release_page(version) -> Url` and `github_page(&Url) -> bool` (scheme `https`, host `github.com`, path under `/lukaszkalnik/sito/`). Unit tests: the page of `0.15.1`; a version with `/`, `?` or `#` stays inside the path; `http`, other hosts and other repositories are refused.
- `update.rs`: command `open_release_page(app, state) -> Result<(), UpdateError>`: `NoUpdate` without a found update; a failing opener → new code `UpdateError::OpenFailed` (logged). Registered in `lib.rs`.
- `update/error.rs`: the `OpenFailed` variant with a serialization test.

### UI (`ui/src/features/update`)

- `api.ts`: `openReleasePage()` wrapper; `'openFailed'` in `UpdateErrorCode` and `errorCodes`.
- `updateReducer.ts`: `installError` becomes `failure: { action: 'install' | 'openPage'; code } | null`, so the message names what failed (a `refactor:` commit first); new event `openPageFailed`.
- `useUpdate.ts`: `openPage` calls `openReleasePage` and dispatches `openPageFailed` on error. The notice stays after a successful click, so „Co nowego?” is still at hand.
- `UpdateNotice.tsx`: for `install === 'manual'`, a first button **„Pobierz”** (where Windows has „Zainstaluj i uruchom ponownie”); the text above stays „Pobierz ją ze strony wydań Sito na GitHubie.”
- `pl.json`: `update.openPage` „Pobierz”, `update.openPageFailed` „Nie udało się otworzyć strony wydania. {{reason}}”, `update.errors.openFailed` „System nie otworzył przeglądarki.”

### Tests

- Vitest: the reducer (`openPageFailed` sets the failure, a new check clears it); `UpdateNotice.test.tsx` (a manual offer shows „Pobierz” and calls the wrapper, a failure shows the alert, an in-app offer has no „Pobierz”).
- Playwright `ui/e2e/update.spec.ts` with `open_release_page` in `tauri-mock.ts` (success or error): a manual offer, „Pobierz”, the command is invoked.

### Docs

- [Updates](../domain/updates.md): the notice table and the macOS install line get „Pobierz”, which opens the release page in the system browser; the network section says the browser, not Sito, loads that page.
- `npm run notices:rust`, commit `ui/notices/rust-crates.txt`.

### Commits

1. `docs:` this plan.
2. `refactor:` `installError` → `failure`.
3. `feat:` open the release page from the update notice (Rust, UI, tests, notices, domain doc).

### Verification

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; `npm run lint`, `npm run typecheck -w ui`, `npm test`, `npm run e2e`.
- On the Mac: a debug build with `SITO_UPDATE_ENDPOINT` pointing at a local `latest.json` that announces a higher version; „Pobierz” opens that version's release page in the browser. Then 🧪 for the user's test.

## Implementation notes

- **Deviation from the plan:** the page is opened with `tauri-plugin-opener`'s free function `tauri_plugin_opener::open_url`, and the plugin is **not registered**. Its Rust API needs no plugin, and without registration the webview does not even get the plugin's (denied) commands. Slice 54 can do the same.
- `src-tauri/src/update/release_page.rs`: `release_page(version) -> Option<Url>` and `github_page(&Url) -> bool` (accepts any page in the repository, its root too).
- UI: the offer's state holds `failure: { action: 'install' | 'openPage'; code }` (separate `refactor:` commit); `UpdateNotice` maps each action to its message with a `Record`, so a new action is a type error there.
- Checks green on macOS: Rust fmt, clippy, tests; frontend lint, typecheck, Vitest, Playwright (60 tests).
- Manual check on the Mac done on 2026-11-08 (see the status line). Note for local tests on macOS: the offered version must be higher than the build's own, so its release page does not exist yet and GitHub shows a 404; the URL is what counts. The `darwin-*` entries in the test `latest.json` need a `url` and `signature`, but the check does not verify the signature.
