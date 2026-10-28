//! Reject commands: move photos into the "Odrzucone" folder and undo it.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use sito_core::actions::{ActionLog, LogError};
use sito_core::reject::{self, RejectError};
use sito_core::timeline::read_photo;
use tauri::{AppHandle, Emitter, State};

use crate::error::CommandError;
use crate::scan::state::{AppState, Scan};

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
    let mut scan = state.scan();
    let scan = scan.as_mut().ok_or(CommandError::Internal)?;
    let rejected = reject_in(scan, &mut actions, id)?;
    notify(&app);
    Ok(rejected)
}

/// [`reject_photo`] on the given scan and action state.
fn reject_in(
    scan: &mut Scan,
    actions: &mut ActionsState,
    id: usize,
) -> Result<RejectedDto, CommandError> {
    let log = actions.log.as_ref().ok_or(CommandError::Internal)?;
    let path = scan.photo(id).ok_or(CommandError::Internal)?.path.clone();

    let rejected = reject::prepare(&path).map_err(move_failed)?;
    // Logged before the move, so a crash in between never leaves an untracked
    // move; undo skips an entry whose photo never arrived.
    let action = log
        .record_reject(&path, &rejected.path, rejected.sidecar.as_ref())
        .map_err(|err| {
            reject::abandon(&rejected);
            log_failed(&err)
        })?;
    if let Err(err) = reject::complete(&path, &rejected) {
        if let Err(err) = log.forget(action) {
            log::warn!("cannot remove the log entry of a failed move: {err}");
        }
        return Err(move_failed(err));
    }
    if rejected.created_dir
        && let Some(dir) = rejected.path.parent()
    {
        actions.created_dirs.insert(dir.to_path_buf());
    }
    scan.remove(id);
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
    let mut scan = state.scan();
    let Some(scan) = scan.as_mut() else {
        return Ok(None);
    };
    let undone = undo_in(scan, &actions)?;
    if undone.is_some() {
        notify(&app);
    }
    Ok(undone)
}

/// [`undo_reject`] on the given scan and action state.
fn undo_in(scan: &mut Scan, actions: &ActionsState) -> Result<Option<UndoneDto>, CommandError> {
    let Some(log) = actions.log.as_ref() else {
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
    let restored = reject::restore(
        &action.to,
        &action.from,
        action.sidecar.as_ref(),
        created_here,
    );
    match restored {
        Ok(restored) => {
            if let Some(err) = restored.sidecar_error {
                log::warn!("photo restored, but not its date file: {err}");
            }
        }
        Err(err) => {
            if matches!(err, RejectError::Missing(_)) {
                // It can never be undone; skip it so the next "Cofnij" goes on.
                log.mark_undone(action.id).map_err(|err| log_failed(&err))?;
            }
            return Err(move_failed(err));
        }
    }
    log.mark_undone(action.id).map_err(|err| log_failed(&err))?;
    let id = scan.restore(&action.from, &action.to, |path| read_photo(path).ok());
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
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;
    use sito_core::series::DEFAULT_MAX_GAP;
    use sito_core::timeline::build_timeline;
    use tempfile::TempDir;

    use super::*;
    use crate::series::series_of;

    /// The two fixture photos 5 s apart, which form the only series.
    const NEWER: &str = "pixel_2024-06-01_12-00-05.jpg";
    const OLDER: &str = "pixel_2024-06-01_12-00-00.jpg";

    /// A temp folder with copies of all EXIF fixtures.
    fn gallery() -> TempDir {
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/exif");
        let dir = TempDir::new().unwrap();
        for entry in fs::read_dir(fixtures).unwrap() {
            let entry = entry.unwrap();
            fs::copy(entry.path(), dir.path().join(entry.file_name())).unwrap();
        }
        dir
    }

    /// A fresh scan of `dir`, like picking the folder in the app.
    fn scan(dir: &Path) -> Scan {
        Scan::new(1, build_timeline(dir, |_| {}).unwrap())
    }

    /// Action state of a freshly started app using the log at `db`.
    fn started(db: &Path) -> ActionsState {
        ActionsState {
            log: Some(ActionLog::open(db).unwrap()),
            created_dirs: HashSet::new(),
        }
    }

    fn id_of(scan: &Scan, name: &str) -> usize {
        scan.photos()
            .find(|(_, photo)| photo.path.ends_with(name))
            .map(|(id, _)| id)
            .unwrap()
    }

    /// File names of each series' photos, newest first.
    fn series_names(scan: &Scan) -> Vec<Vec<String>> {
        series_of(scan, DEFAULT_MAX_GAP)
            .iter()
            .map(|s| {
                scan.timeline.photos[s.start..s.start + s.len]
                    .iter()
                    .map(|p| file_name(&p.path))
                    .collect()
            })
            .collect()
    }

    #[test]
    fn rejecting_and_undoing_moves_the_photo_and_keeps_the_series() {
        let dir = gallery();
        let db = dir.path().join("sito.db");
        let mut scan = scan(dir.path());
        let mut actions = started(&db);
        let id = id_of(&scan, NEWER);

        let rejected = reject_in(&mut scan, &mut actions, id).unwrap();

        assert_eq!(rejected.name, NEWER);
        assert!(!dir.path().join(NEWER).exists());
        assert!(dir.path().join("Odrzucone").join(NEWER).exists());
        assert!(scan.photo(id).is_none());
        // The kept photo is still a series of its own.
        assert_eq!(series_names(&scan), [[OLDER]]);

        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();

        assert_eq!(undone.id, Some(id));
        assert_eq!(undone.name, NEWER);
        assert!(dir.path().join(NEWER).exists());
        // Created by this session and empty again: removed.
        assert!(!dir.path().join("Odrzucone").exists());
        assert_eq!(series_names(&scan), [[NEWER, OLDER]]);
        assert_eq!(undo_in(&mut scan, &actions).unwrap(), None);
    }

    #[test]
    fn undo_keeps_an_odrzucone_folder_that_existed_before() {
        let dir = gallery();
        fs::create_dir(dir.path().join("Odrzucone")).unwrap();
        let mut scan = scan(dir.path());
        let mut actions = started(&dir.path().join("sito.db"));
        let id = id_of(&scan, NEWER);

        reject_in(&mut scan, &mut actions, id).unwrap();
        undo_in(&mut scan, &actions).unwrap().unwrap();

        assert!(dir.path().join(NEWER).exists());
        assert!(dir.path().join("Odrzucone").is_dir());
    }

    #[test]
    fn series_and_undo_survive_a_restart() {
        let dir = gallery();
        let db = dir.path().join("sito.db");
        {
            let mut scan = scan(dir.path());
            let mut actions = started(&db);
            let id = id_of(&scan, NEWER);
            reject_in(&mut scan, &mut actions, id).unwrap();
        }

        // Restart: a new scan finds the rejected photo in "Odrzucone".
        let mut scan = scan(dir.path());
        let actions = started(&db);
        assert_eq!(scan.timeline.rejected.len(), 1);
        assert!(scan.photos().all(|(_, p)| !p.path.ends_with(NEWER)));
        assert_eq!(series_names(&scan), [[OLDER]]);

        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();

        assert_eq!(undone.name, NEWER);
        assert!(undone.id.is_some());
        assert!(dir.path().join(NEWER).exists());
        assert_eq!(scan.timeline.rejected, Vec::new());
        assert_eq!(series_names(&scan), [[NEWER, OLDER]]);
        // Created before this session: kept, although empty.
        assert!(dir.path().join("Odrzucone").is_dir());
    }

    #[test]
    fn a_rejected_photo_that_is_gone_is_reported_and_skipped() {
        let dir = gallery();
        let mut scan = scan(dir.path());
        let mut actions = started(&dir.path().join("sito.db"));
        let older = id_of(&scan, OLDER);
        let newer = id_of(&scan, NEWER);
        reject_in(&mut scan, &mut actions, older).unwrap();
        reject_in(&mut scan, &mut actions, newer).unwrap();
        // Someone moved the newest rejected photo away.
        fs::remove_file(dir.path().join("Odrzucone").join(NEWER)).unwrap();

        assert_eq!(undo_in(&mut scan, &actions), Err(CommandError::Missing));
        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();

        assert_eq!(undone.name, OLDER);
        assert!(dir.path().join(OLDER).exists());
    }

    #[test]
    fn a_move_interrupted_after_logging_is_skipped_by_undo() {
        let dir = gallery();
        let mut scan = scan(dir.path());
        let actions = started(&dir.path().join("sito.db"));
        // The app stopped after logging the rejection, before moving the photo.
        let photo = dir.path().join(NEWER);
        let log = actions.log.as_ref().unwrap();
        log.record_reject(&photo, &dir.path().join("Odrzucone").join(NEWER), None)
            .unwrap();

        assert_eq!(undo_in(&mut scan, &actions), Err(CommandError::Missing));
        assert_eq!(undo_in(&mut scan, &actions), Ok(None));
        assert!(photo.exists());
    }

    #[test]
    fn refuses_to_reject_without_an_action_log() {
        let dir = gallery();
        let mut scan = scan(dir.path());
        let mut actions = ActionsState {
            log: None,
            created_dirs: HashSet::new(),
        };
        let id = id_of(&scan, NEWER);

        assert_eq!(
            reject_in(&mut scan, &mut actions, id),
            Err(CommandError::Internal)
        );
        assert!(dir.path().join(NEWER).exists());
        assert!(scan.photo(id).is_some());
    }

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
