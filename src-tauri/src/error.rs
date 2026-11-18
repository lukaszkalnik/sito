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
    /// A move would overwrite a file that took the target name meanwhile.
    Occupied,
    /// The photo is no longer at its path: moved, renamed or deleted
    /// outside Sito.
    PhotoMissing,
    /// The rejected photo is no longer in the rejected folder.
    Missing,
    /// The rejected photo is no longer in the trash.
    MissingFromTrash,
    /// The photo's drive has no trash (removable and network drives), so
    /// moving it there would delete it.
    NoTrash,
    /// The „Odrzucone” folder is a link to another place.
    RejectedDirIsLink,
    /// This system cannot restore files from the trash.
    TrashUndoUnsupported,
    /// A later opening of a folder began before this one finished; its
    /// result is the one shown.
    Superseded,
    /// A series edit no longer fits the series (they changed meanwhile,
    /// e.g. in another window).
    SeriesChanged,
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
