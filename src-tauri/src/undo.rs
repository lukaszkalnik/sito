//! „Cofnij”: undoes the newest action in the open folder, a rejection or a
//! series edit, in one history.

use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use sito_core::actions::{Rejections, Undoable};
use sito_core::series::DEFAULT_MAX_GAP;
use tauri::{AppHandle, Manager, State};

use crate::error::{CommandError, off_main_thread};
use crate::reject::{
    Actions, ActionsState, RestoredDto, TIMELINE_CHANGED, announce, log_failed, restore_rejection,
};
use crate::scan::state::{AppState, Scan};
use crate::series::edit::{EditUndoneDto, undo_edit};

/// What „Cofnij” took back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UndoneDto {
    /// A rejected photo came back.
    Rejection(RestoredDto),
    /// A series edit was taken back.
    Edit(EditUndoneDto),
}

/// Whether „Cofnij” has something to undo in the current folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UndoStateDto {
    /// A rejection or a series edit can be undone.
    Available,
    /// Nothing is left to undo.
    #[serde(rename = "none")]
    Nothing,
    /// Only rejections into a trash this system cannot restore from are left.
    TrashOnly,
}

/// Undoes the newest rejection or series edit in the current folder; `None`
/// if there is nothing to undo. The series are those for `max_gap_seconds`
/// (`None`: the default gap), which decide the photos an undone edit
/// reports as changed. Runs off the main thread: the trash can take a
/// moment.
#[tauri::command]
pub async fn undo(
    max_gap_seconds: Option<u32>,
    app: AppHandle,
) -> Result<Option<UndoneDto>, CommandError> {
    let max_gap = max_gap_seconds.map_or(DEFAULT_MAX_GAP, |s| Duration::from_secs(s.into()));
    off_main_thread(move || {
        let actions = app.state::<Actions>().inner().lock();
        let state = app.state::<AppState>();
        let mut scan = state.scan();
        let Some(scan) = scan.as_mut() else {
            return Ok(None);
        };
        let undone = undo_in(scan, &actions, max_gap)?;
        if undone.is_some() {
            announce(&app, TIMELINE_CHANGED);
        }
        Ok(undone)
    })
    .await?
}

/// [`undo`] on the given scan and action state.
pub(crate) fn undo_in(
    scan: &mut Scan,
    actions: &ActionsState,
    max_gap: Duration,
) -> Result<Option<UndoneDto>, CommandError> {
    let Some(log) = actions.opened_log() else {
        return Ok(None);
    };
    let action = log
        .last_undoable(&scan.timeline.root, actions.undoable_rejections())
        .map_err(|err| log_failed(&err))?;
    action
        .map(|action| match action {
            Undoable::Reject(action) => {
                restore_rejection(scan, actions, log, &action).map(UndoneDto::Rejection)
            }
            Undoable::Edit(action) => undo_edit(scan, log, &action, max_gap).map(UndoneDto::Edit),
        })
        .transpose()
}

/// Whether there is something to undo in the current folder.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn get_undo_state(state: State<'_, AppState>, actions: State<'_, Actions>) -> UndoStateDto {
    let actions = actions.lock();
    let scan = state.scan();
    scan.as_ref().map_or(UndoStateDto::Nothing, |scan| {
        undo_state(&actions, &scan.timeline.root)
    })
}

/// [`get_undo_state`] for the folder `root`.
pub(crate) fn undo_state(actions: &ActionsState, root: &Path) -> UndoStateDto {
    let Some(log) = actions.opened_log() else {
        return UndoStateDto::Nothing;
    };
    let offered = |rejections| {
        log.last_undoable(root, rejections)
            .is_ok_and(|action| action.is_some())
    };
    if offered(actions.undoable_rejections()) {
        UndoStateDto::Available
    } else if offered(Rejections::All) {
        UndoStateDto::TrashOnly
    } else {
        UndoStateDto::Nothing
    }
}

#[cfg(test)]
mod tests;
