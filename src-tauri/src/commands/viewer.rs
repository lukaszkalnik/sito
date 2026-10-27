//! Viewer commands: show a photo full size in a second, reusable window.

use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use super::scan::CommandError;
use crate::window_fit;

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
/// receive [`SHOW_EVENT`] yet.
#[derive(Debug, Default)]
pub struct ViewerState {
    pending: Mutex<Option<ViewerRequest>>,
}

impl ViewerState {
    fn set(&self, request: ViewerRequest) {
        *self.pending.lock().unwrap_or_else(PoisonError::into_inner) = Some(request);
    }

    fn take(&self) -> Option<ViewerRequest> {
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

/// Shows a photo in the viewer window, opening the window if needed.
// Creating a window from a synchronous command can deadlock on Windows.
#[tauri::command]
pub async fn open_viewer(
    request: ViewerRequest,
    app: AppHandle,
    viewer: State<'_, ViewerState>,
) -> Result<(), CommandError> {
    // Kept until the window takes it, in case it is still loading.
    viewer.set(request);
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
            &app,
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
        window_fit::show_placed(&window);
        window.set_focus().map_err(|err| internal("focus", &err))?;
    }
    Ok(())
}

/// The request the viewer window should show once it has loaded, if any.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn take_viewer_request(viewer: State<'_, ViewerState>) -> Option<ViewerRequest> {
    viewer.take()
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
    fn pending_request_is_taken_once() {
        let state = ViewerState::default();
        state.set(request());
        state.set(ViewerRequest { id: 9, ..request() });

        assert_eq!(state.take(), Some(ViewerRequest { id: 9, ..request() }));
        assert_eq!(state.take(), None);
    }
}
