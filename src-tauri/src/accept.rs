//! Accept commands: mark photos as accepted (the keepers of a series), take
//! the mark off, and list the accepted photos of the current scan.

use std::path::{Path, PathBuf};

use serde::Serialize;
use sito_core::accept::{AcceptAction, AcceptChange};
use sito_core::actions::ActionLog;
use tauri::{AppHandle, Manager};

use crate::error::{CommandError, off_main_thread};
use crate::reject::{Actions, UndoneDto, UndoneKind, announce, file_name, log_failed};
use crate::scan::state::{AppState, Scan};

/// Event sent to all windows when a photo is accepted or loses its mark.
pub const ACCEPTED_CHANGED: &str = "accepted-changed";

/// A photo whose mark changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedDto {
    /// File name of the photo.
    name: String,
}

/// Marks photo `id` of the current scan as accepted, and logs it.
#[tauri::command]
pub async fn accept_photo(id: usize, app: AppHandle) -> Result<AcceptedDto, CommandError> {
    change_mark(app, id, AcceptChange::Accepted).await
}

/// Takes the accepted mark off photo `id` of the current scan, and logs it.
#[tauri::command]
pub async fn unaccept_photo(id: usize, app: AppHandle) -> Result<AcceptedDto, CommandError> {
    change_mark(app, id, AcceptChange::Unaccepted).await
}

async fn change_mark(
    app: AppHandle,
    id: usize,
    change: AcceptChange,
) -> Result<AcceptedDto, CommandError> {
    off_main_thread(move || {
        let actions = app.state::<Actions>().inner().lock();
        let path = photo_path(&app, id)?;
        let changed = set_mark(actions.log()?, &path, change)?;
        announce(&app, ACCEPTED_CHANGED);
        Ok(changed)
    })
    .await?
}

/// The path of photo `id` of the current scan.
fn photo_path(app: &AppHandle, id: usize) -> Result<PathBuf, CommandError> {
    let state = app.state::<AppState>();
    let scan = state.scan();
    let photo = scan.as_ref().and_then(|scan| scan.photo(id));
    Ok(photo.ok_or(CommandError::Internal)?.path.clone())
}

/// Changes the mark of the photo at `path`. A photo already in that state
/// (e.g. changed in another window) is left as it is.
pub(crate) fn set_mark(
    log: &ActionLog,
    path: &Path,
    change: AcceptChange,
) -> Result<AcceptedDto, CommandError> {
    log.set_accepted(path, change)
        .map_err(|err| log_failed(&err))?;
    Ok(AcceptedDto {
        name: file_name(path),
    })
}

/// Timeline ids of the accepted photos of the current scan.
#[tauri::command]
pub async fn get_accepted_ids(app: AppHandle) -> Result<Vec<usize>, CommandError> {
    off_main_thread(move || {
        let state = app.state::<AppState>();
        let Some(root) = state.scan().as_ref().map(|scan| scan.timeline.root.clone()) else {
            return Ok(Vec::new());
        };
        let paths = {
            let actions = app.state::<Actions>().inner().lock();
            let Ok(log) = actions.log() else {
                return Ok(Vec::new());
            };
            log.accepted_paths(&root).map_err(|err| log_failed(&err))?
        };
        Ok(state
            .scan()
            .as_ref()
            .map(|scan| accepted_ids(scan, &paths))
            .unwrap_or_default())
    })
    .await?
}

/// Ids of the photos of `scan` whose path is in `paths`, in timeline order.
pub(crate) fn accepted_ids(scan: &Scan, paths: &std::collections::HashSet<PathBuf>) -> Vec<usize> {
    scan.photos()
        .filter(|(_, photo)| paths.contains(&photo.path))
        .map(|(id, _)| id)
        .collect()
}

/// Reverts the accept or un-accept `action`.
pub(crate) fn undo(
    scan: &Scan,
    log: &ActionLog,
    action: &AcceptAction,
) -> Result<UndoneDto, CommandError> {
    log.undo_accept(action).map_err(|err| log_failed(&err))?;
    let kind = match action.change {
        AcceptChange::Accepted => UndoneKind::Unaccepted,
        AcceptChange::Unaccepted => UndoneKind::Reaccepted,
    };
    let id = scan
        .photos()
        .find(|(_, photo)| photo.path == action.path)
        .map(|(id, _)| id);
    Ok(UndoneDto {
        id,
        name: file_name(&action.path),
        kind,
    })
}
