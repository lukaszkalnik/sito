//! The error codes of the cache settings' commands.

use serde::Serialize;
use sito_core::cache_dir::{MoveError, RefusedFolder};

use super::gate::Busy;

/// Why a cache command failed. Only a code is sent; the UI shows a
/// translated message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "camelCase")]
pub enum CacheError {
    /// Files cannot be created in the chosen folder.
    NotWritable,
    /// The chosen folder is inside the folder open in Sito.
    InsideOpenFolder,
    /// The chosen folder is the cache folder, inside it or around it.
    InsideCache,
    /// The chosen folder is a symbolic link or a junction.
    Link,
    /// The chosen folder's name cannot be stored.
    UnsupportedName,
    /// A folder is being scanned or refreshed; try again once it is done.
    Busy,
    /// A file of the cache could not be copied or renamed.
    CopyFailed,
    /// A copy differed from its original.
    CopyDiffers,
    /// The user stopped the move.
    Cancelled,
    /// The folder could not be shown in the file manager.
    RevealFailed,
    /// Unexpected failure (the setting could not be saved, a task failed).
    Internal,
}

impl From<RefusedFolder> for CacheError {
    fn from(refused: RefusedFolder) -> Self {
        match refused {
            RefusedFolder::NotWritable => Self::NotWritable,
            RefusedFolder::InsideOpenFolder => Self::InsideOpenFolder,
            RefusedFolder::InsideCache => Self::InsideCache,
            RefusedFolder::Link => Self::Link,
            RefusedFolder::UnsupportedName => Self::UnsupportedName,
        }
    }
}

impl From<MoveError> for CacheError {
    fn from(err: MoveError) -> Self {
        match err {
            MoveError::NotWritable { .. } => Self::NotWritable,
            MoveError::Io { .. } => Self::CopyFailed,
            MoveError::Mismatch(_) => Self::CopyDiffers,
            MoveError::Stopped => Self::Cancelled,
        }
    }
}

impl From<Busy> for CacheError {
    fn from(Busy: Busy) -> Self {
        Self::Busy
    }
}

impl From<crate::error::CommandError> for CacheError {
    fn from(_: crate::error::CommandError) -> Self {
        // `off_main_thread` fails only when its task panicked.
        Self::Internal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_sent_as_codes() {
        assert_eq!(
            serde_json::to_value(CacheError::InsideOpenFolder).unwrap(),
            serde_json::json!({ "code": "insideOpenFolder" })
        );
    }

    #[test]
    fn a_stopped_move_was_cancelled() {
        assert_eq!(CacheError::from(MoveError::Stopped), CacheError::Cancelled);
    }
}
