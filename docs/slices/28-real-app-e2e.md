# Slice 28 – End-to-end tests of the real app

**Status:** 📝 draft (concept; detailed plan when the slice starts)

## Context

The Playwright tests in `ui/e2e/` run the UI in a plain browser against a fake `window.__TAURI_INTERNALS__` (`ui/e2e/tauri-mock.ts`). They test the UI flows well, but no Rust code runs: there are no real windows, no plugins, no main thread and no event loop. Two bugs in slice 7 lived in exactly that gap:

- **v0.6.2** froze when opening the viewer. `open_viewer` placed the new window from its command thread while holding the window-state plugin's lock, and the plugin's window registration on the main thread waited for that lock. The mock just returns `null` for `open_viewer`, so the e2e tests never saw it.
- **v0.6.3** forgot window sizes across restarts, because closing the last window on Windows did not reach the plugin's save on exit. No test closes and restarts the real app.

As a stopgap, the release smoke test now opens the viewer in the installed app and closes it again (`SITO_SMOKE_TEST=1`, `src-tauri/src/smoke.rs`). It fails on a freeze, a viewer that does not appear, or a missing window state file. On Linux under Xvfb it reproduces the v0.6.2 freeze. But it only runs on releases and tests a single flow.

## Concept

Drive the built app, not a mock, with WebDriver through [`tauri-driver`](https://v2.tauri.app/develop/tests/webdriver/), and run it on every pull request.

- **Where:** `tauri-driver` supports Windows (Edge WebDriver for WebView2) and Linux (`WebKitWebDriver`), but not macOS (WKWebView has no driver). CI runs it on `windows-latest`, the platform users have, and on `ubuntu-latest` under Xvfb, which is cheaper and still reproduced the v0.6.2 freeze. macOS keeps the smoke test until slice 23.
- **Cheap first step:** run the existing smoke mode on pull requests on `ubuntu-latest` (debug build with `custom-protocol`, `xvfb-run`, 60 s timeout). This catches window-opening freezes before a release, not after.
- **Test runner:** WebdriverIO with Mocha, the setup the Tauri docs show, in a separate `e2e-app/` workspace so the fast mocked Playwright suite stays as it is. Alternative to evaluate: Playwright `connectOverCDP` to WebView2 (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=…`). It reuses Playwright and its locators but works on Windows only and drives only one webview per connection.
- **Isolation:** each test gets a fresh copy of `fixtures/` in a temp folder and its own app data, config and cache folders (`APPDATA`/`LOCALAPPDATA` on Windows, `XDG_*` on Linux), so tests never touch the user's real folders and rejecting photos is safe.
- **Opening a folder:** the native folder dialog cannot be driven over WebDriver. A test-only start parameter (e.g. `SITO_E2E_FOLDER`) opens that folder at start, like reopening the last folder (slice 9) does. Like `SITO_SMOKE_TEST`, it only exists for tests.
- **Several windows:** WebDriver lists both webviews as window handles, so a test can click a photo in the main window, switch to the viewer and check it.

## First tests

1. Open `fixtures/` → the photo count and series match the known groups.
2. Click a photo → the viewer window opens within a few seconds and shows that photo; clicking another tile reuses the window.
3. ← → in the viewer step through the series.
4. Reject in the viewer → the file is in `Odrzucone` on disk; undo → it is back.
5. Resize and move both windows, close the app like a user, start it again → the folder reopens from the cache, and both windows come back at their size and position (the v0.6.3 case).

## Open questions

- Build time on CI: a debug build of `src-tauri` per job, with `Swatinem/rust-cache`. Run on every pull request, or only when `src-tauri/`, `ui/` or `crates/` change?
- Whether the Edge WebDriver version can be matched to the runner's WebView2 automatically (`msedgedriver-tool`) or must be pinned.
- Whether a debug-only Tauri command for the tests is better than the environment variable for opening a folder.
