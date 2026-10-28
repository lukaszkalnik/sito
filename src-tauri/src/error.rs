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
    /// The rejected photo is no longer in the rejected folder.
    Missing,
    /// Unexpected failure inside the app.
    Internal,
}
