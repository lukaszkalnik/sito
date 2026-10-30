//! Viewer commands: show a photo full size in a second, reusable window.

use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::error::CommandError;
use crate::window_fit;

pub mod page;
pub mod photo_protocol;

/// Label of the viewer window.
pub const LABEL: &str = "viewer";
/// Event that tells an open viewer window which photo to show.
pub const SHOW_EVENT: &str = "viewer-show";

/// Which photos the viewer steps through; mirrors the timeline view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ViewMode {
    /// Only photos in series.
    Series,
    /// All photos.
    All,
}

/// What the viewer should show, sent by the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerRequest {
    /// Timeline id of the photo to show.
    pub id: usize,
    /// Scan the id belongs to.
    pub scan_id: u64,
    /// View of the main window, which previous/next follow.
    pub mode: ViewMode,
    /// Time gap of the series, in seconds.
    pub gap: u32,
}

/// The last request, for a viewer window that is still loading and so cannot
/// receive [`SHOW_EVENT`] yet. It is kept until a newer one replaces it, so a
/// viewer page that is reloaded after reading it still finds it.
#[derive(Debug, Default)]
pub struct ViewerState {
    current: Mutex<Option<ViewerRequest>>,
}

impl ViewerState {
    fn set(&self, request: ViewerRequest) {
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = Some(request);
    }

    fn get(&self) -> Option<ViewerRequest> {
        *self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Shows a photo in the viewer window, opening the window if needed.
// Creating a window from a synchronous command can deadlock on Windows.
#[tauri::command]
pub async fn open_viewer(request: ViewerRequest, app: AppHandle) -> Result<(), CommandError> {
    show(&app, request)
}

/// Shows `request` in the viewer window, creating and placing the window if
/// needed. Must not be called on the main thread, which it waits for.
pub fn show(app: &AppHandle, request: ViewerRequest) -> Result<(), CommandError> {
    // Kept for the window in case it is still loading.
    app.state::<ViewerState>().set(request);
    if let Some(window) = app.get_webview_window(LABEL) {
        app.emit_to(LABEL, SHOW_EVENT, request)
            .map_err(|err| internal("emit", &err))?;
        window
            .unminimize()
            .map_err(|err| internal("unminimize", &err))?;
        window.show().map_err(|err| internal("show", &err))?;
        window.set_focus().map_err(|err| internal("focus", &err))?;
    } else {
        // Created hidden and shown once placed, so it never visibly jumps.
        let window = WebviewWindowBuilder::new(
            app,
            LABEL,
            WebviewUrl::App("index.html?window=viewer".into()),
        )
        .title("Sito")
        .inner_size(1200.0, 800.0)
        .min_inner_size(480.0, 360.0)
        .center()
        .visible(false)
        .build()
        .map_err(|err| internal("open", &err))?;
        // Placed on the main thread, after the window-state plugin has registered
        // the window there. Doing it from this thread deadlocks: it would hold the
        // plugin's lock while waiting for the main thread, which waits for the lock.
        let placed = window.clone();
        window
            .run_on_main_thread(move || {
                window_fit::show_placed(&placed);
                if let Err(err) = placed.set_focus() {
                    log::warn!("cannot focus the viewer window: {err}");
                }
            })
            .map_err(|err| internal("place", &err))?;
    }
    Ok(())
}

/// The last request for the viewer, which a viewer window shows once it has
/// loaded; `None` before the first one.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn current_viewer_request(viewer: State<'_, ViewerState>) -> Option<ViewerRequest> {
    viewer.get()
}

fn internal(action: &str, err: &tauri::Error) -> CommandError {
    log::warn!("viewer window: {action} failed: {err}");
    CommandError::Internal
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn request() -> ViewerRequest {
        ViewerRequest {
            id: 7,
            scan_id: 1_700_000_000_000,
            mode: ViewMode::Series,
            gap: 10,
        }
    }

    #[test]
    fn request_round_trips_as_camel_case() {
        let value =
            json!({ "id": 7, "scanId": 1_700_000_000_000_u64, "mode": "series", "gap": 10 });

        assert_eq!(serde_json::to_value(request()).unwrap(), value);
        assert_eq!(
            serde_json::from_value::<ViewerRequest>(
                json!({ "id": 1, "scanId": 2, "mode": "all", "gap": 5 })
            )
            .unwrap()
            .mode,
            ViewMode::All
        );
    }

    #[test]
    fn the_latest_request_stays_until_a_newer_one_replaces_it() {
        let state = ViewerState::default();
        assert_eq!(state.get(), None);
        state.set(request());
        state.set(ViewerRequest { id: 9, ..request() });

        // A viewer page reloaded after reading it gets it again.
        assert_eq!(state.get(), Some(ViewerRequest { id: 9, ..request() }));
        assert_eq!(state.get(), Some(ViewerRequest { id: 9, ..request() }));

        state.set(request());
        assert_eq!(state.get(), Some(request()));
    }
}
