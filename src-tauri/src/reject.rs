//! Reject commands: move photos into the "Odrzucone" folder or the system
//! trash, and undo the newest action (a rejection or an accept).

mod drive;
#[cfg(test)]
mod fake_bin;
mod folder;
mod trash;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use sito_core::actions::{ActionLog, LogError, LoggedAction, Placement, RejectAction};
use sito_core::reject::RejectError;
use sito_core::timeline::read_photo;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::accept::{self, ACCEPTED_CHANGED};
use crate::error::{CommandError, off_main_thread};
use crate::scan::state::{AppState, Scan};
use trash::{Bin, SystemBin};

/// Event sent to all windows when photos leave or return to the timeline.
pub const TIMELINE_CHANGED: &str = "timeline-changed";

/// What rejecting and accepting need beyond the scan.
#[derive(Debug)]
pub struct Actions(Mutex<ActionsState>);

#[derive(Debug)]
pub(crate) struct ActionsState {
    /// The action log, or `None` if its database could not be opened;
    /// rejecting is then refused, since it could not be undone.
    log: Option<ActionLog>,
    /// "Odrzucone" folders created since the app started. Only these are
    /// removed again once undo leaves them empty; folders that were there
    /// before stay.
    created_dirs: HashSet<PathBuf>,
    /// The system trash.
    bin: Box<dyn Bin>,
}

impl Actions {
    pub fn new(log: Option<ActionLog>) -> Self {
        Self(Mutex::new(ActionsState {
            log,
            created_dirs: HashSet::new(),
            bin: Box::new(SystemBin),
        }))
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, ActionsState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ActionsState {
    /// The action log, or [`CommandError::Internal`] if it could not be opened.
    pub(crate) fn log(&self) -> Result<&ActionLog, CommandError> {
        self.log.as_ref().ok_or(CommandError::Internal)
    }
}

/// Where a rejected photo goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Destination {
    /// The "Odrzucone" subfolder of the photo's folder.
    Folder,
    /// The system trash.
    Trash,
}

/// A rejected photo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedDto {
    /// File name of the photo.
    name: String,
}

/// The action "Cofnij" undid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoneDto {
    /// Timeline id of the photo, or `None` if it could not be read back or
    /// is not on the timeline.
    pub(crate) id: Option<usize>,
    /// File name of the photo.
    pub(crate) name: String,
    /// What undoing did to the photo.
    pub(crate) kind: UndoneKind,
}

/// What undoing an action did to its photo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UndoneKind {
    /// A rejected photo is back where it was.
    Restored,
    /// An accepted photo lost its mark.
    Unaccepted,
    /// A photo whose mark was taken off is accepted again.
    Reaccepted,
}

impl UndoneKind {
    /// The event announcing the change to all windows.
    fn event(self) -> &'static str {
        match self {
            Self::Restored => TIMELINE_CHANGED,
            Self::Unaccepted | Self::Reaccepted => ACCEPTED_CHANGED,
        }
    }
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

/// Moves photo `id` of the current scan to `destination`, logs it, and takes
/// it off the timeline. Runs off the main thread: the trash can take a moment.
#[tauri::command]
pub async fn reject_photo(
    id: usize,
    destination: Destination,
    app: AppHandle,
) -> Result<RejectedDto, CommandError> {
    off_main_thread(move || {
        let mut actions = app.state::<Actions>().inner().lock();
        let state = app.state::<AppState>();
        let mut scan = state.scan();
        let scan = scan.as_mut().ok_or(CommandError::Internal)?;
        let rejected = reject_in(scan, &mut actions, id, destination)?;
        announce(&app, TIMELINE_CHANGED);
        Ok(rejected)
    })
    .await?
}

/// [`reject_photo`] on the given scan and action state.
fn reject_in(
    scan: &mut Scan,
    actions: &mut ActionsState,
    id: usize,
    destination: Destination,
) -> Result<RejectedDto, CommandError> {
    let log = actions.log.as_ref().ok_or(CommandError::Internal)?;
    let path = scan.photo(id).ok_or(CommandError::Internal)?.path.clone();

    match destination {
        Destination::Folder => folder::reject(log, &path, &mut actions.created_dirs)?,
        Destination::Trash => trash::reject(log, actions.bin.as_ref(), &path)?,
    }
    scan.remove(id);
    Ok(RejectedDto {
        name: file_name(&path),
    })
}

/// Undoes the newest action in the current folder: moves a rejected photo
/// back onto the timeline, or reverts an accept or un-accept; `None` if there
/// is nothing to undo. Runs off the main thread: the trash can take a moment.
#[tauri::command]
pub async fn undo_last(app: AppHandle) -> Result<Option<UndoneDto>, CommandError> {
    off_main_thread(move || {
        let actions = app.state::<Actions>().inner().lock();
        let state = app.state::<AppState>();
        let mut scan = state.scan();
        let Some(scan) = scan.as_mut() else {
            return Ok(None);
        };
        let undone = undo_in(scan, &actions)?;
        if let Some(undone) = &undone {
            announce(&app, undone.kind.event());
        }
        Ok(undone)
    })
    .await?
}

/// [`undo_last`] on the given scan and action state.
fn undo_in(scan: &mut Scan, actions: &ActionsState) -> Result<Option<UndoneDto>, CommandError> {
    let Some(log) = actions.log.as_ref() else {
        return Ok(None);
    };
    let action = log
        .last_undoable(&scan.timeline.root)
        .map_err(|err| log_failed(&err))?;
    match action {
        None => Ok(None),
        Some(LoggedAction::Reject(action)) => undo_reject(scan, actions, log, &action).map(Some),
        Some(LoggedAction::Accept(action)) => accept::undo(scan, log, &action).map(Some),
    }
}

/// Moves the photo of the rejection `action` back and puts it on the timeline.
fn undo_reject(
    scan: &mut Scan,
    actions: &ActionsState,
    log: &ActionLog,
    action: &RejectAction,
) -> Result<UndoneDto, CommandError> {
    let (undone, rejected_path) = match &action.place {
        Placement::Folder { to, extra } => (
            folder::undo(&action.from, to, extra, &actions.created_dirs),
            to.as_path(),
        ),
        Placement::Trash { files, at } => (
            trash::undo(actions.bin.as_ref(), files, *at),
            action.from.as_path(),
        ),
    };
    if let Err(err) = undone {
        if matches!(
            err,
            CommandError::Missing
                | CommandError::MissingFromTrash
                | CommandError::TrashUndoUnsupported
        ) {
            // It can never be undone here; skip it so the next "Cofnij" goes on.
            log.mark_undone(action.id).map_err(|err| log_failed(&err))?;
        }
        return Err(err);
    }
    log.mark_undone(action.id).map_err(|err| log_failed(&err))?;
    let id = scan.restore(&action.from, rejected_path, |path| read_photo(path).ok());
    Ok(UndoneDto {
        id,
        name: file_name(&action.from),
        kind: UndoneKind::Restored,
    })
}

/// Whether there is an action to undo in the current folder.
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

/// Sends `event` to all windows.
pub(crate) fn announce(app: &AppHandle, event: &str) {
    if let Err(err) = app.emit(event, ()) {
        log::warn!("cannot announce {event}: {err}");
    }
}

fn move_failed(err: RejectError) -> CommandError {
    log::warn!("{err}");
    err.into()
}

pub(crate) fn log_failed(err: &LogError) -> CommandError {
    log::warn!("{err}");
    CommandError::Internal
}

pub(crate) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;
    use sito_core::accept::AcceptChange;
    use sito_core::series::{DEFAULT_MAX_GAP, Grouping};
    use sito_core::timeline::build_timeline;
    use sito_core::trash::TrashError;
    use tempfile::TempDir;

    use super::*;
    use crate::reject::fake_bin::FakeBin;

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
            bin: Box::new(FakeBin::new(Path::new("no-bin"))),
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
        let (all, removed) = scan.photos_with_removed();
        Grouping::new(&all, &removed, DEFAULT_MAX_GAP)
            .series
            .iter()
            .map(|s| {
                s.members
                    .iter()
                    .map(|&m| file_name(&scan.timeline.photos[m].path))
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

        let rejected = reject_in(&mut scan, &mut actions, id, Destination::Folder).unwrap();

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

        reject_in(&mut scan, &mut actions, id, Destination::Folder).unwrap();
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
            reject_in(&mut scan, &mut actions, id, Destination::Folder).unwrap();
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
        reject_in(&mut scan, &mut actions, older, Destination::Folder).unwrap();
        reject_in(&mut scan, &mut actions, newer, Destination::Folder).unwrap();
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
        log.record_reject(&photo, &dir.path().join("Odrzucone").join(NEWER), &[])
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
            bin: Box::new(FakeBin::new(Path::new("no-bin"))),
        };
        let id = id_of(&scan, NEWER);

        assert_eq!(
            reject_in(&mut scan, &mut actions, id, Destination::Folder),
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
                name: "IMG_1.jpg".into(),
                kind: UndoneKind::Reaccepted,
            })
            .unwrap(),
            json!({ "id": 3, "name": "IMG_1.jpg", "kind": "reaccepted" })
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

    /// Action state using the log at `db` and a fake trash in `bin`.
    fn started_with_bin(db: &Path, bin: FakeBin) -> ActionsState {
        ActionsState {
            bin: Box::new(bin),
            ..started(db)
        }
    }

    #[test]
    fn rejecting_into_the_trash_and_undoing_brings_the_photo_back() {
        let dir = gallery();
        let bin = TempDir::new().unwrap();
        let mut scan = scan(dir.path());
        let db = bin.path().join("sito.db");
        let mut actions = started_with_bin(&db, FakeBin::new(bin.path()));
        let id = id_of(&scan, NEWER);

        let rejected = reject_in(&mut scan, &mut actions, id, Destination::Trash).unwrap();

        assert_eq!(rejected.name, NEWER);
        assert!(!dir.path().join(NEWER).exists());
        assert!(bin.path().join(NEWER).exists());
        assert!(!dir.path().join("Odrzucone").exists());
        assert!(scan.photo(id).is_none());

        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();

        assert_eq!(undone.id, Some(id));
        assert!(dir.path().join(NEWER).exists());
        assert_eq!(series_names(&scan), [[NEWER, OLDER]]);
        assert_eq!(undo_in(&mut scan, &actions).unwrap(), None);
    }

    #[test]
    fn undo_from_the_trash_works_after_a_restart() {
        let dir = gallery();
        let bin = TempDir::new().unwrap();
        let db = bin.path().join("sito.db");
        {
            let mut scan = scan(dir.path());
            let mut actions = started_with_bin(&db, FakeBin::new(bin.path()));
            let id = id_of(&scan, NEWER);
            reject_in(&mut scan, &mut actions, id, Destination::Trash).unwrap();
        }

        let mut scan = scan(dir.path());
        let actions = started_with_bin(&db, FakeBin::new(bin.path()));
        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();

        assert_eq!(undone.name, NEWER);
        assert!(undone.id.is_some());
        assert!(dir.path().join(NEWER).exists());
    }

    #[test]
    fn refuses_the_trash_on_a_drive_without_one() {
        let dir = gallery();
        let bin = TempDir::new().unwrap();
        let mut scan = scan(dir.path());
        let mut fake = FakeBin::new(bin.path());
        fake.has_trash = false;
        let mut actions = started_with_bin(&bin.path().join("sito.db"), fake);
        let id = id_of(&scan, NEWER);

        assert_eq!(
            reject_in(&mut scan, &mut actions, id, Destination::Trash),
            Err(CommandError::NoTrash)
        );
        assert!(dir.path().join(NEWER).exists());
        assert!(scan.photo(id).is_some());
        assert_eq!(undo_in(&mut scan, &actions).unwrap(), None);
    }

    #[test]
    fn a_photo_gone_from_the_trash_is_reported_and_skipped() {
        let dir = gallery();
        let bin = TempDir::new().unwrap();
        let mut scan = scan(dir.path());
        let mut actions = started_with_bin(&bin.path().join("sito.db"), FakeBin::new(bin.path()));
        let older = id_of(&scan, OLDER);
        let newer = id_of(&scan, NEWER);
        reject_in(&mut scan, &mut actions, older, Destination::Folder).unwrap();
        reject_in(&mut scan, &mut actions, newer, Destination::Trash).unwrap();
        // The trash was emptied.
        fs::remove_file(bin.path().join(NEWER)).unwrap();

        assert_eq!(
            undo_in(&mut scan, &actions),
            Err(CommandError::MissingFromTrash)
        );
        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();

        assert_eq!(undone.name, OLDER);
        assert!(dir.path().join(OLDER).exists());
    }

    #[test]
    fn a_trash_move_that_cannot_be_undone_here_is_reported_and_skipped() {
        let dir = gallery();
        let bin = TempDir::new().unwrap();
        let mut scan = scan(dir.path());
        let mut fake = FakeBin::new(bin.path());
        fake.can_restore = false;
        let mut actions = started_with_bin(&bin.path().join("sito.db"), fake);
        let id = id_of(&scan, NEWER);
        reject_in(&mut scan, &mut actions, id, Destination::Trash).unwrap();

        assert_eq!(
            undo_in(&mut scan, &actions),
            Err(CommandError::TrashUndoUnsupported)
        );
        assert_eq!(undo_in(&mut scan, &actions), Ok(None));
    }

    #[test]
    fn destinations_deserialize_from_camel_case() {
        assert_eq!(
            serde_json::from_value::<Destination>(json!("folder")).unwrap(),
            Destination::Folder
        );
        assert_eq!(
            serde_json::from_value::<Destination>(json!("trash")).unwrap(),
            Destination::Trash
        );
    }

    #[test]
    fn trash_errors_map_to_codes() {
        let path = PathBuf::from("IMG_1.jpg");
        let cases = [
            (TrashError::Occupied(path.clone()), "occupied"),
            (TrashError::Missing(path), "missingFromTrash"),
            (TrashError::RestoreUnsupported, "trashUndoUnsupported"),
        ];
        for (err, code) in cases {
            assert_eq!(
                serde_json::to_value(CommandError::from(err)).unwrap(),
                json!({ "code": code })
            );
        }
        assert_eq!(
            serde_json::to_value(CommandError::NoTrash).unwrap(),
            json!({ "code": "noTrash" })
        );
    }

    /// Rejects `OLDER` to `first` and `NEWER` to `second`, then undoes twice.
    /// Undo takes no destination: each rejection comes back the way it went,
    /// whatever destination is chosen meanwhile.
    fn reject_both_ways_and_undo(first: Destination, second: Destination) {
        let dir = gallery();
        let bin = TempDir::new().unwrap();
        let mut scan = scan(dir.path());
        let mut actions = started_with_bin(&bin.path().join("sito.db"), FakeBin::new(bin.path()));
        let older = id_of(&scan, OLDER);
        let newer = id_of(&scan, NEWER);
        reject_in(&mut scan, &mut actions, older, first).unwrap();
        reject_in(&mut scan, &mut actions, newer, second).unwrap();
        let place_of = |destination, name| match destination {
            Destination::Folder => dir.path().join("Odrzucone").join(name),
            Destination::Trash => bin.path().join(name),
        };
        assert!(place_of(first, OLDER).exists());
        assert!(place_of(second, NEWER).exists());

        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();
        assert_eq!(undone.name, NEWER);
        assert!(dir.path().join(NEWER).exists());
        assert!(!dir.path().join(OLDER).exists());

        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();
        assert_eq!(undone.name, OLDER);
        assert!(dir.path().join(OLDER).exists());
        assert_eq!(undo_in(&mut scan, &actions).unwrap(), None);
        // Created by this session and empty again: removed.
        assert!(!dir.path().join("Odrzucone").exists());
        assert_eq!(series_names(&scan), [[NEWER, OLDER]]);
    }

    fn accepted_names(scan: &Scan, actions: &ActionsState) -> Vec<String> {
        let paths = actions
            .log()
            .unwrap()
            .accepted_paths(&scan.timeline.root)
            .unwrap();
        accept::accepted_ids(scan, &paths)
            .into_iter()
            .map(|id| file_name(&scan.photo(id).unwrap().path))
            .collect()
    }

    fn set_mark(scan: &Scan, actions: &ActionsState, name: &str, change: AcceptChange) {
        let path = scan.photo(id_of(scan, name)).unwrap().path.clone();
        accept::set_mark(actions.log().unwrap(), &path, change).unwrap();
    }

    #[test]
    fn undo_takes_back_a_rejection_and_then_the_accept_before_it() {
        let dir = gallery();
        let mut scan = scan(dir.path());
        let mut actions = started(&dir.path().join("sito.db"));
        let id = id_of(&scan, NEWER);
        set_mark(&scan, &actions, NEWER, AcceptChange::Accepted);
        assert_eq!(accepted_names(&scan, &actions), [NEWER]);
        reject_in(&mut scan, &mut actions, id, Destination::Folder).unwrap();
        assert_eq!(accepted_names(&scan, &actions), Vec::<String>::new());

        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();
        assert_eq!(undone.kind, UndoneKind::Restored);
        // Back from "Odrzucone", still accepted.
        assert_eq!(accepted_names(&scan, &actions), [NEWER]);

        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();
        assert_eq!(
            undone,
            UndoneDto {
                id: Some(id),
                name: NEWER.into(),
                kind: UndoneKind::Unaccepted,
            }
        );
        assert_eq!(accepted_names(&scan, &actions), Vec::<String>::new());
        assert_eq!(undo_in(&mut scan, &actions).unwrap(), None);
    }

    #[test]
    fn undoing_an_unaccept_accepts_the_photo_again() {
        let dir = gallery();
        let mut scan = scan(dir.path());
        let actions = started(&dir.path().join("sito.db"));
        set_mark(&scan, &actions, OLDER, AcceptChange::Accepted);
        set_mark(&scan, &actions, OLDER, AcceptChange::Unaccepted);

        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();

        assert_eq!(undone.kind, UndoneKind::Reaccepted);
        assert_eq!(accepted_names(&scan, &actions), [OLDER]);
    }

    #[test]
    fn accepted_photos_survive_a_restart() {
        let dir = gallery();
        let db = dir.path().join("sito.db");
        {
            let scan = scan(dir.path());
            let actions = started(&db);
            set_mark(&scan, &actions, OLDER, AcceptChange::Accepted);
        }

        let scan = scan(dir.path());
        let actions = started(&db);

        assert_eq!(accepted_names(&scan, &actions), [OLDER]);
    }

    #[test]
    fn undoes_a_trash_rejection_and_then_a_folder_one() {
        reject_both_ways_and_undo(Destination::Folder, Destination::Trash);
    }

    #[test]
    fn undoes_a_folder_rejection_and_then_a_trash_one() {
        reject_both_ways_and_undo(Destination::Trash, Destination::Folder);
    }
}
