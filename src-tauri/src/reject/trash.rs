//! Rejecting into the system trash, and restoring from it.

use std::fmt::Debug;
use std::path::{Path, PathBuf};

use sito_core::actions::ActionLog;
use sito_core::reject::PhotoFiles;
use sito_core::trash::{self, RestoredFromTrash, TrashError};

use super::{drive, log_failed};
use crate::error::CommandError;

/// The system trash, behind a trait so tests never touch the real one.
pub(super) trait Bin: Send + Debug {
    /// Whether files in `folder` can go into the trash (and not be deleted
    /// for good, as on drives without one).
    fn has_trash(&self, folder: &Path) -> bool;
    /// Moves `files` (a photo first) into the trash.
    fn put(&self, files: &[PathBuf]) -> Result<(), TrashError>;
    /// Moves `files` (a photo first) back, as moved at `at` (Unix seconds).
    fn take_back(&self, files: &[PathBuf], at: i64) -> Result<RestoredFromTrash, TrashError>;
}

/// The trash of the operating system.
#[derive(Debug)]
pub(super) struct SystemBin;

impl Bin for SystemBin {
    fn has_trash(&self, folder: &Path) -> bool {
        drive::has_trash(folder)
    }

    fn put(&self, files: &[PathBuf]) -> Result<(), TrashError> {
        trash::move_to_trash(files)
    }

    fn take_back(&self, files: &[PathBuf], at: i64) -> Result<RestoredFromTrash, TrashError> {
        trash::restore_from_trash(files, at)
    }
}

impl From<TrashError> for CommandError {
    fn from(err: TrashError) -> Self {
        match err {
            TrashError::Missing(_) => Self::Missing,
            TrashError::Occupied(_) => Self::Occupied,
            TrashError::RestoreUnsupported => Self::TrashUndoUnsupported,
            TrashError::Trash { .. } | TrashError::Restore(_) => Self::Io,
        }
    }
}

/// Moves the photo at `path` and its other files into the trash, logging
/// the move first. Refused with [`CommandError::NoTrash`] on a drive
/// without a trash, where the system would delete the files instead.
pub(super) fn reject(log: &ActionLog, bin: &dyn Bin, path: &Path) -> Result<(), CommandError> {
    let folder = path.parent().ok_or(CommandError::Io)?;
    if !bin.has_trash(folder) {
        return Err(CommandError::NoTrash);
    }
    let files = PhotoFiles::find(path);
    // Logged before the move, like a move into the folder.
    let action = log.record_trash(&files).map_err(|err| log_failed(&err))?;
    if let Err(err) = bin.put(&files.all()) {
        if let Err(err) = log.forget(action) {
            log::warn!("cannot remove the log entry of a failed move: {err}");
        }
        return Err(trash_failed(err));
    }
    Ok(())
}

/// Moves the photo `files` back from the trash, as moved at `at`.
pub(super) fn undo(bin: &dyn Bin, files: &[PathBuf], at: i64) -> Result<(), CommandError> {
    let restored = bin.take_back(files, at).map_err(trash_failed)?;
    for err in restored.errors {
        log::warn!("photo restored, but not this file of it: {err}");
    }
    Ok(())
}

fn trash_failed(err: TrashError) -> CommandError {
    log::warn!("{err}");
    err.into()
}
