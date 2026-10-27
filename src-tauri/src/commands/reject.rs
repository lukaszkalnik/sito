//! Reject commands: move photos into the "Odrzucone" folder and undo it.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use sito_core::actions::{ActionLog, LogError};
use sito_core::reject::{self, RejectError};
use sito_core::timeline::read_photo;
use tauri::{AppHandle, Emitter, State};

use super::scan::CommandError;
use crate::state::AppState;

/// Event sent to all windows when photos leave or return to the timeline.
pub const TIMELINE_CHANGED: &str = "timeline-changed";

/// What rejecting needs beyond the scan.
#[derive(Debug)]
pub struct Actions(Mutex<ActionsState>);

#[derive(Debug)]
struct ActionsState {
    /// The action log, or `None` if its database could not be opened;
    /// rejecting is then refused, since it could not be undone.
    log: Option<ActionLog>,
    /// "Odrzucone" folders created since the app started. Only these are
    /// removed again once undo leaves them empty; folders that were there
    /// before stay.
    created_dirs: HashSet<PathBuf>,
}

impl Actions {
    pub fn new(log: Option<ActionLog>) -> Self {
        Self(Mutex::new(ActionsState {
            log,
            created_dirs: HashSet::new(),
        }))
    }

    fn lock(&self) -> MutexGuard<'_, ActionsState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A photo moved into the rejected folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedDto {
    /// File name of the photo.
    name: String,
}

/// A photo moved back from the rejected folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoneDto {
    /// Timeline id of the photo, or `None` if it could not be read back.
    id: Option<usize>,
    /// File name of the photo.
    name: String,
}

/// Whether "Cofnij" has something to undo in the current folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoStateDto {
    can_undo: bool,
}

impl From<RejectError> for CommandError {
    fn from(err: RejectError) -> Self {
        match err {
            RejectError::Occupied(_) => Self::Occupied,
            RejectError::Missing(_) => Self::Missing,
            RejectError::NoParent(_) | RejectError::Io { .. } => Self::Io,
        }
    }
}

/// Moves photo `id` of the current scan into the "Odrzucone" subfolder of its
/// folder, logs it, and takes it off the timeline.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn reject_photo(
    id: usize,
    app: AppHandle,
    state: State<'_, AppState>,
    actions: State<'_, Actions>,
) -> Result<RejectedDto, CommandError> {
    let mut actions = actions.lock();
    let actions = &mut *actions;
    let log = actions.log.as_ref().ok_or(CommandError::Internal)?;
    let mut scan = state.scan();
    let scan = scan.as_mut().ok_or(CommandError::Internal)?;
    let path = scan.photo(id).ok_or(CommandError::Internal)?.path.clone();

    let rejected = reject::reject(&path).map_err(move_failed)?;
    if let Err(err) = log.record_reject(&path, &rejected.path) {
        // Without a log entry it could not be undone: move it back.
        log::warn!("cannot log the rejection of {}: {err}", path.display());
        reject::restore(&rejected.path, &path, rejected.created_dir).map_err(move_failed)?;
        return Err(CommandError::Internal);
    }
    if rejected.created_dir
        && let Some(dir) = rejected.path.parent()
    {
        actions.created_dirs.insert(dir.to_path_buf());
    }
    scan.remove(id);
    notify(&app);
    Ok(RejectedDto {
        name: file_name(&path),
    })
}

/// Moves the most recently rejected photo of the current folder back and puts
/// it on the timeline again; `None` if there is nothing to undo.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn undo_reject(
    app: AppHandle,
    state: State<'_, AppState>,
    actions: State<'_, Actions>,
) -> Result<Option<UndoneDto>, CommandError> {
    let actions = actions.lock();
    let Some(log) = actions.log.as_ref() else {
        return Ok(None);
    };
    let mut scan = state.scan();
    let Some(scan) = scan.as_mut() else {
        return Ok(None);
    };
    let action = log
        .last_undoable(&scan.timeline.root)
        .map_err(|err| log_failed(&err))?;
    let Some(action) = action else {
        return Ok(None);
    };

    let created_here = action
        .to
        .parent()
        .is_some_and(|dir| actions.created_dirs.contains(dir));
    if let Err(err) = reject::restore(&action.to, &action.from, created_here) {
        if matches!(err, RejectError::Missing(_)) {
            // It can never be undone; skip it so the next "Cofnij" goes on.
            log.mark_undone(action.id).map_err(|err| log_failed(&err))?;
        }
        return Err(move_failed(err));
    }
    log.mark_undone(action.id).map_err(|err| log_failed(&err))?;
    let id = scan.restore(&action.from, |path| read_photo(path).ok());
    notify(&app);
    Ok(Some(UndoneDto {
        id,
        name: file_name(&action.from),
    }))
}

/// Whether there is a rejection to undo in the current folder.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn get_undo_state(state: State<'_, AppState>, actions: State<'_, Actions>) -> UndoStateDto {
    let actions = actions.lock();
    let scan = state.scan();
    let can_undo = match (actions.log.as_ref(), scan.as_ref()) {
        (Some(log), Some(scan)) => log
            .last_undoable(&scan.timeline.root)
            .is_ok_and(|action| action.is_some()),
        _ => false,
    };
    UndoStateDto { can_undo }
}

fn notify(app: &AppHandle) {
    if let Err(err) = app.emit(TIMELINE_CHANGED, ()) {
        log::warn!("cannot announce the timeline change: {err}");
    }
}

fn move_failed(err: RejectError) -> CommandError {
    log::warn!("{err}");
    err.into()
}

fn log_failed(err: &LogError) -> CommandError {
    log::warn!("{err}");
    CommandError::Internal
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;

    #[test]
    fn results_serialize_as_camel_case() {
        assert_eq!(
            serde_json::to_value(RejectedDto {
                name: "IMG_1.jpg".into()
            })
            .unwrap(),
            json!({ "name": "IMG_1.jpg" })
        );
        assert_eq!(
            serde_json::to_value(UndoneDto {
                id: Some(3),
                name: "IMG_1.jpg".into()
            })
            .unwrap(),
            json!({ "id": 3, "name": "IMG_1.jpg" })
        );
        assert_eq!(
            serde_json::to_value(UndoStateDto { can_undo: true }).unwrap(),
            json!({ "canUndo": true })
        );
    }

    #[test]
    fn reject_errors_map_to_codes() {
        let path = PathBuf::from("IMG_1.jpg");
        let cases = [
            (RejectError::Occupied(path.clone()), "occupied"),
            (RejectError::Missing(path.clone()), "missing"),
            (RejectError::NoParent(path), "io"),
        ];
        for (err, code) in cases {
            assert_eq!(
                serde_json::to_value(CommandError::from(err)).unwrap(),
                json!({ "code": code })
            );
        }
    }
}
