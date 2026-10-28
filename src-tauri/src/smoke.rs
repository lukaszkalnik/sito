//! Smoke test of the installed app, run by the release workflow. With
//! `SITO_SMOKE_TEST=1`, Sito opens the viewer like a click on a photo does,
//! closes both windows like a user, and exits. This covers what the e2e tests
//! cannot, because they mock the Tauri layer: a freeze while the viewer opens
//! (as in v0.6.2) and the window state not being written on close.

use std::ffi::OsStr;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::viewer::{self, ViewMode, ViewerRequest};

/// Environment variable that turns the smoke test on.
pub const ENV: &str = "SITO_SMOKE_TEST";
/// Exit code when the viewer window could not be opened.
const EXIT_OPEN_FAILED: i32 = 2;
/// Exit code when the viewer window was opened but is not visible.
const EXIT_NOT_SHOWN: i32 = 3;
/// How long the main window gets to load before the viewer opens.
const LOAD_TIME: Duration = Duration::from_secs(3);
/// How long the viewer stays open before both windows close.
const SHOW_TIME: Duration = Duration::from_secs(2);

/// Whether `value` of [`ENV`] turns the smoke test on.
#[must_use]
pub fn enabled(value: Option<&OsStr>) -> bool {
    value == Some(OsStr::new("1"))
}

/// Starts the smoke test on its own thread, as the viewer command runs off the
/// main thread too.
pub fn spawn(app: AppHandle) {
    thread::spawn(move || run(&app));
}

fn run(app: &AppHandle) {
    thread::sleep(LOAD_TIME);
    let request = ViewerRequest {
        id: 0,
        scan_id: 0,
        mode: ViewMode::Series,
        gap: 10,
    };
    if viewer::show(app, request).is_err() {
        app.exit(EXIT_OPEN_FAILED);
        return;
    }
    thread::sleep(SHOW_TIME);
    // Queued on the main thread after the viewer's placement, so the viewer is
    // shown by then unless placing it failed.
    let handle = app.clone();
    if let Err(err) = app.run_on_main_thread(move || close_all(&handle)) {
        log::warn!("smoke test: cannot close the windows: {err}");
        app.exit(EXIT_OPEN_FAILED);
    }
}

/// Closes every window like a user would, which saves the window state and
/// exits the app with code 0 once the last one is gone.
fn close_all(app: &AppHandle) {
    let shown = app
        .get_webview_window(viewer::LABEL)
        .and_then(|window| window.is_visible().ok())
        == Some(true);
    if !shown {
        app.exit(EXIT_NOT_SHOWN);
        return;
    }
    for window in app.webview_windows().values() {
        if let Err(err) = window.close() {
            log::warn!(
                "smoke test: cannot close the {} window: {err}",
                window.label()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_1_turns_the_smoke_test_on() {
        assert!(enabled(Some(OsStr::new("1"))));
        assert!(!enabled(None));
        assert!(!enabled(Some(OsStr::new(""))));
        assert!(!enabled(Some(OsStr::new("0"))));
        assert!(!enabled(Some(OsStr::new("true"))));
    }
}
