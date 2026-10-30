//! The error every command returns to the UI.

use serde::Serialize;

/// Error sent to the UI. Only a code is sent; the UI shows a translated message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "camelCase")]
pub enum CommandError {
    /// The folder does not exist.
    NotFound,
    /// The path is not a folder.
    NotADirectory,
    /// The folder could not be read.
    Io,
    /// Undo would overwrite a file that now has the photo's original name.
    Occupied,
    /// The rejected photo is no longer in the rejected folder or the trash.
    Missing,
    /// The photo's drive has no trash (removable and network drives), so
    /// moving it there would delete it.
    NoTrash,
    /// This system cannot restore files from the trash.
    TrashUndoUnsupported,
    /// Unexpected failure inside the app.
    Internal,
}

/// Runs `work` on a blocking thread, off the main thread that also handles
/// the window, so a command with a lot to do never freezes the app.
///
/// # Errors
///
/// [`CommandError::Internal`] if `work` panicked.
pub async fn off_main_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|err| {
            log::error!("background task failed: {err}");
            CommandError::Internal
        })
}
