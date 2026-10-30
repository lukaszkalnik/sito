//! Rejecting into the "Odrzucone" subfolder of the photo's folder, and
//! moving the photo back from there.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use sito_core::actions::{ActionLog, RejectAction};
use sito_core::reject;

use super::{log_failed, move_failed};
use crate::error::CommandError;

/// Moves the photo at `path` and its other files into "Odrzucone", logging
/// the move first. A folder created for it is added to `created_dirs`.
pub(super) fn reject(
    log: &ActionLog,
    path: &Path,
    created_dirs: &mut HashSet<PathBuf>,
) -> Result<(), CommandError> {
    let rejected = reject::prepare(path).map_err(move_failed)?;
    // Logged before the move, so a crash in between never leaves an untracked
    // move; undo skips an entry whose photo never arrived.
    let action = log
        .record_reject(path, &rejected.path, &rejected.extra)
        .map_err(|err| {
            reject::abandon(&rejected);
            log_failed(&err)
        })?;
    if let Err(err) = reject::complete(path, &rejected) {
        if let Err(err) = log.forget(action) {
            log::warn!("cannot remove the log entry of a failed move: {err}");
        }
        return Err(move_failed(err));
    }
    if rejected.created_dir
        && let Some(dir) = rejected.path.parent()
    {
        created_dirs.insert(dir.to_path_buf());
    }
    Ok(())
}

/// Moves the photo of `action` and its other files back from "Odrzucone".
/// The folder is removed once empty only if it is in `created_dirs`.
pub(super) fn undo(
    action: &RejectAction,
    created_dirs: &HashSet<PathBuf>,
) -> Result<(), CommandError> {
    let created_here = action
        .to
        .parent()
        .is_some_and(|dir| created_dirs.contains(dir));
    let restored = reject::restore(&action.to, &action.from, &action.extra, created_here)
        .map_err(move_failed)?;
    for err in restored.errors {
        log::warn!("photo restored, but not this file of it: {err}");
    }
    Ok(())
}
