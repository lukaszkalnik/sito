//! Reject commands: move photos into the rejected folder or the system
//! trash, and bring a rejected photo back (for „Cofnij”, see `undo`).

#[cfg(test)]
mod fake_bin;
mod folder;
mod trash;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use sito_core::actions::{ActionLog, LogError, Placement, RejectAction, Rejections};
use sito_core::moved::MovedPhoto;
use sito_core::reject::RejectError;
use sito_core::scan::RejectedDirName;
use sito_core::timeline::read_photo;
use sito_core::trash::TrashRestore;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::{CommandError, off_main_thread};
use crate::language::LanguageState;
use crate::scan::state::{AppState, Scan};
use trash::{Bin, SystemBin};

/// Event sent to all windows when photos leave or return to the timeline.
pub const TIMELINE_CHANGED: &str = "timeline-changed";

/// What rejecting, undoing and accepting need beyond the scan.
#[derive(Debug)]
pub struct Actions(Mutex<ActionsState>);

#[derive(Debug)]
pub(crate) struct ActionsState {
    /// The action log, or `None` if its database could not be opened;
    /// rejecting is then refused, since it could not be undone.
    log: Option<ActionLog>,
    /// Rejected folders created since the app started. Only these are
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

    /// The action log, if it could be opened.
    pub(crate) fn opened_log(&self) -> Option<&ActionLog> {
        self.log.as_ref()
    }

    /// The rejections undo may offer: trash moves only where the trash can
    /// be restored from. Skipped ones stay in the log for a later version.
    pub(crate) fn undoable_rejections(&self) -> Rejections {
        match self.bin.restore() {
            TrashRestore::Supported => Rejections::All,
            TrashRestore::Unsupported => Rejections::FolderOnly,
        }
    }
}

/// Where the user chose rejected photos to go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Destination {
    /// The rejected subfolder of the photo's folder.
    Folder,
    /// The system trash.
    Trash,
}

/// Where a rejected photo goes, with the name of a rejected folder to
/// create.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RejectTarget {
    /// The rejected subfolder of the photo's folder: one already there, or a
    /// new one with this name.
    Folder(RejectedDirName),
    /// The system trash.
    Trash,
}

impl Destination {
    /// The target of this destination when a new rejected folder is named
    /// `new_dir`.
    const fn target(self, new_dir: RejectedDirName) -> RejectTarget {
        match self {
            Self::Folder => RejectTarget::Folder(new_dir),
            Self::Trash => RejectTarget::Trash,
        }
    }
}

/// A rejected photo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedDto {
    /// File name of the photo.
    name: String,
}

/// The rejected photo "Cofnij" brought back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoredDto {
    /// Timeline id of the photo, or `None` if it could not be read back or
    /// is not on the timeline.
    pub(crate) id: Option<usize>,
    /// File name of the photo.
    pub(crate) name: String,
    /// The file name it came back under, when its own name was taken by
    /// another file; `None` when it got its own name back.
    pub(crate) restored_name: Option<String>,
}

impl From<RejectError> for CommandError {
    fn from(err: RejectError) -> Self {
        match err {
            RejectError::Occupied(_) => Self::Occupied,
            RejectError::Missing(_) => Self::Missing,
            RejectError::PhotoGone(_) => Self::PhotoMissing,
            RejectError::RejectedDirIsLink(_) => Self::RejectedDirIsLink,
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
        let new_dir = app
            .state::<LanguageState>()
            .current()
            .language
            .rejected_dir();
        let rejected = reject_in(scan, &mut actions, id, destination.target(new_dir))?;
        announce(&app, TIMELINE_CHANGED);
        Ok(rejected)
    })
    .await?
}

/// [`reject_photo`] on the given scan and action state.
pub(crate) fn reject_in(
    scan: &mut Scan,
    actions: &mut ActionsState,
    id: usize,
    target: RejectTarget,
) -> Result<RejectedDto, CommandError> {
    let log = actions.log.as_ref().ok_or(CommandError::Internal)?;
    let path = scan.photo(id).ok_or(CommandError::Internal)?.path.clone();
    // Checked before anything is logged; the UI offers a refresh.
    if !path.is_file() {
        return Err(CommandError::PhotoMissing);
    }

    match target {
        RejectTarget::Folder(new_dir) => {
            folder::reject(log, &path, new_dir, &mut actions.created_dirs)?;
        }
        RejectTarget::Trash => trash::reject(log, actions.bin.as_ref(), &path)?,
    }
    scan.remove(id);
    Ok(RejectedDto {
        name: file_name(&path),
    })
}

/// Moves the photo of the rejection `action` back and puts it on the timeline.
pub(crate) fn restore_rejection(
    scan: &mut Scan,
    actions: &ActionsState,
    log: &ActionLog,
    action: &RejectAction,
) -> Result<RestoredDto, CommandError> {
    let (undone, rejected_path) = match &action.place {
        Placement::Folder { to, extra } => (
            folder::undo(&action.from, to, extra, &actions.created_dirs),
            to.as_path(),
        ),
        Placement::Trash { files, at } => (
            trash::undo(actions.bin.as_ref(), files, *at).map(|()| action.from.clone()),
            action.from.as_path(),
        ),
    };
    let restored = match undone {
        Ok(restored) => restored,
        Err(err) => {
            // A refused trash restore (`TrashUndoUnsupported`) is not skipped: a
            // later version may be able to restore it.
            if matches!(err, CommandError::Missing | CommandError::MissingFromTrash) {
                // It can never be undone; skip it so the next "Cofnij" goes on.
                log.mark_undone(action.id).map_err(|err| log_failed(&err))?;
            }
            return Err(err);
        }
    };
    log.mark_undone(action.id).map_err(|err| log_failed(&err))?;
    let renamed = restored != action.from;
    if renamed {
        follow_mark(log, &action.from, &restored);
        follow_pin(scan, log, &action.from, &restored);
    }
    let id = scan.restore(&action.from, &restored, rejected_path, |path| {
        read_photo(path).ok()
    });
    Ok(RestoredDto {
        id,
        name: file_name(&action.from),
        restored_name: renamed.then(|| file_name(&restored)),
    })
}

/// Moves the accepted mark of a photo restored under a new name along.
fn follow_mark(log: &ActionLog, from: &Path, to: &Path) {
    let moved = MovedPhoto {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
    };
    if let Err(err) = log.move_accepted(&[moved]) {
        log::warn!("cannot move the accepted mark of a restored photo: {err}");
    }
}

/// Moves the series edit pin of a photo restored under a new name along,
/// in the log and in `scan`.
fn follow_pin(scan: &mut Scan, log: &ActionLog, from: &Path, to: &Path) {
    let moved = MovedPhoto {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
    };
    let pins = log
        .move_pins(&[moved])
        .and_then(|()| log.pins_under(&scan.timeline.root));
    match pins {
        Ok(pins) => scan.set_pins(pins),
        Err(err) => log::warn!("cannot move the series edit of a restored photo: {err}"),
    }
}

/// Sends `event` to all windows.
pub(crate) fn announce(app: &AppHandle, event: &str) {
    announce_with(app, event, ());
}

/// Sends `event` with `payload` to every window.
pub(crate) fn announce_with(app: &AppHandle, event: &str, payload: impl Serialize + Clone) {
    if let Err(err) = app.emit(event, payload) {
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
pub(crate) mod tests {
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;
    use sito_core::accept::AcceptChange;
    use sito_core::series::{DEFAULT_MAX_GAP, Grouping, Pins};
    use sito_core::timeline::build_timeline;
    use sito_core::trash::TrashError;
    use tempfile::TempDir;

    use super::*;
    use crate::accept;
    use crate::reject::fake_bin::FakeBin;
    use crate::undo::{UndoStateDto, UndoneDto, undo_state};

    /// The two fixture photos 5 s apart, which form the only series.
    pub(crate) const NEWER: &str = "pixel_2024-06-01_12-00-05.jpg";
    pub(crate) const OLDER: &str = "pixel_2024-06-01_12-00-00.jpg";
    /// Rejecting into `Odrzucone` unless the folder has another rejected folder.
    pub(crate) const FOLDER: RejectTarget = RejectTarget::Folder(RejectedDirName::ODRZUCONE);

    /// Undoes the newest action, which must be a rejection (or nothing).
    fn undo_in(
        scan: &mut Scan,
        actions: &ActionsState,
    ) -> Result<Option<RestoredDto>, CommandError> {
        crate::undo::undo_in(scan, actions, DEFAULT_MAX_GAP).map(|undone| {
            undone.map(|undone| match undone {
                UndoneDto::Rejection(restored) => restored,
                UndoneDto::Edit(edit) => panic!("not a rejection: {edit:?}"),
            })
        })
    }

    /// A temp folder with copies of all EXIF fixtures.
    pub(crate) fn gallery() -> TempDir {
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/exif");
        let dir = TempDir::new().unwrap();
        for entry in fs::read_dir(fixtures).unwrap() {
            let entry = entry.unwrap();
            fs::copy(entry.path(), dir.path().join(entry.file_name())).unwrap();
        }
        dir
    }

    /// A fresh scan of `dir`, like picking the folder in the app.
    pub(crate) fn scan(dir: &Path) -> Scan {
        Scan::new(1, build_timeline(dir, |_| {}).unwrap())
    }

    /// Action state of a freshly started app using the log at `db`.
    pub(crate) fn started(db: &Path) -> ActionsState {
        ActionsState {
            log: Some(ActionLog::open(db).unwrap()),
            created_dirs: HashSet::new(),
            bin: Box::new(FakeBin::new(Path::new("no-bin"))),
        }
    }

    pub(crate) fn id_of(scan: &Scan, name: &str) -> usize {
        scan.photos()
            .find(|(_, photo)| photo.path.ends_with(name))
            .map(|(id, _)| id)
            .unwrap()
    }

    /// File names of each series' photos, newest first.
    fn series_names(scan: &Scan) -> Vec<Vec<String>> {
        let (all, removed) = scan.photos_with_removed();
        Grouping::new(&all, &removed, DEFAULT_MAX_GAP, &Pins::default())
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

        let rejected = reject_in(&mut scan, &mut actions, id, FOLDER).unwrap();

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

        reject_in(&mut scan, &mut actions, id, FOLDER).unwrap();
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
            reject_in(&mut scan, &mut actions, id, FOLDER).unwrap();
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
        reject_in(&mut scan, &mut actions, older, FOLDER).unwrap();
        reject_in(&mut scan, &mut actions, newer, FOLDER).unwrap();
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
            reject_in(&mut scan, &mut actions, id, FOLDER),
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
            serde_json::to_value(RestoredDto {
                id: Some(3),
                name: "IMG_1.jpg".into(),
                restored_name: Some("IMG_1 (2).jpg".into()),
            })
            .unwrap(),
            json!({ "id": 3, "name": "IMG_1.jpg", "restoredName": "IMG_1 (2).jpg" })
        );
    }

    #[test]
    fn reject_errors_map_to_codes() {
        let path = PathBuf::from("IMG_1.jpg");
        let cases = [
            (RejectError::Occupied(path.clone()), "occupied"),
            (RejectError::Missing(path.clone()), "missing"),
            (RejectError::PhotoGone(path.clone()), "photoMissing"),
            (
                RejectError::RejectedDirIsLink(path.clone()),
                "rejectedDirIsLink",
            ),
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

        let rejected = reject_in(&mut scan, &mut actions, id, RejectTarget::Trash).unwrap();

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
            reject_in(&mut scan, &mut actions, id, RejectTarget::Trash).unwrap();
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
            reject_in(&mut scan, &mut actions, id, RejectTarget::Trash),
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
        reject_in(&mut scan, &mut actions, older, FOLDER).unwrap();
        reject_in(&mut scan, &mut actions, newer, RejectTarget::Trash).unwrap();
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
    fn without_trash_restore_undo_skips_trash_moves() {
        let dir = gallery();
        let bin = TempDir::new().unwrap();
        let mut scan = scan(dir.path());
        let mut fake = FakeBin::new(bin.path());
        fake.restore = TrashRestore::Unsupported;
        let mut actions = started_with_bin(&bin.path().join("sito.db"), fake);
        let root = scan.timeline.root.clone();
        let older = id_of(&scan, OLDER);
        let newer = id_of(&scan, NEWER);
        reject_in(&mut scan, &mut actions, older, FOLDER).unwrap();
        reject_in(&mut scan, &mut actions, newer, RejectTarget::Trash).unwrap();
        assert_eq!(undo_state(&actions, &root), UndoStateDto::Available);

        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();

        assert_eq!(undone.name, OLDER);
        assert!(dir.path().join(OLDER).exists());
        assert!(bin.path().join(NEWER).exists());
        assert_eq!(undo_state(&actions, &root), UndoStateDto::TrashOnly);
        assert_eq!(undo_in(&mut scan, &actions), Ok(None));
        // Still in the log, for a version that can restore it.
        assert_eq!(undo_state(&actions, &root), UndoStateDto::TrashOnly);
    }

    #[test]
    fn undo_state_follows_the_log() {
        let dir = gallery();
        let mut scan = scan(dir.path());
        let mut actions = started(&dir.path().join("sito.db"));
        let root = scan.timeline.root.clone();
        assert_eq!(undo_state(&actions, &root), UndoStateDto::Nothing);

        let id = id_of(&scan, NEWER);
        reject_in(&mut scan, &mut actions, id, FOLDER).unwrap();
        assert_eq!(undo_state(&actions, &root), UndoStateDto::Available);

        undo_in(&mut scan, &actions).unwrap();
        assert_eq!(undo_state(&actions, &root), UndoStateDto::Nothing);
    }

    #[test]
    fn a_rejection_after_a_language_switch_goes_into_the_same_folder() {
        let dir = gallery();
        let mut scan = scan(dir.path());
        let mut actions = started(&dir.path().join("sito.db"));
        let older = id_of(&scan, OLDER);
        let newer = id_of(&scan, NEWER);
        let english = RejectTarget::Folder(RejectedDirName::REJECTED);

        reject_in(&mut scan, &mut actions, older, FOLDER).unwrap();
        reject_in(&mut scan, &mut actions, newer, english).unwrap();

        assert!(dir.path().join("Odrzucone").join(OLDER).exists());
        assert!(dir.path().join("Odrzucone").join(NEWER).exists());
        assert!(!dir.path().join("Rejected").exists());
        assert_eq!(undo_in(&mut scan, &actions).unwrap().unwrap().name, NEWER);
        assert_eq!(undo_in(&mut scan, &actions).unwrap().unwrap().name, OLDER);
        assert!(dir.path().join(OLDER).exists() && dir.path().join(NEWER).exists());
        // Created by this session and empty again: removed.
        assert!(!dir.path().join("Odrzucone").exists());
    }

    #[test]
    fn a_new_rejected_folder_is_named_in_the_language_and_removed_on_undo() {
        let dir = gallery();
        let mut scan = scan(dir.path());
        let mut actions = started(&dir.path().join("sito.db"));
        let id = id_of(&scan, NEWER);

        let german = RejectTarget::Folder(RejectedDirName::ABGELEHNT);
        reject_in(&mut scan, &mut actions, id, german).unwrap();

        assert!(dir.path().join("Abgelehnt").join(NEWER).exists());
        undo_in(&mut scan, &actions).unwrap();
        assert!(dir.path().join(NEWER).exists());
        assert!(!dir.path().join("Abgelehnt").exists());
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
    fn the_folder_destination_names_a_new_folder_and_the_trash_does_not() {
        let name = RejectedDirName::ABGELEHNT;
        assert_eq!(Destination::Folder.target(name), RejectTarget::Folder(name));
        assert_eq!(Destination::Trash.target(name), RejectTarget::Trash);
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
    fn reject_both_ways_and_undo(first: RejectTarget, second: RejectTarget) {
        let dir = gallery();
        let bin = TempDir::new().unwrap();
        let mut scan = scan(dir.path());
        let mut actions = started_with_bin(&bin.path().join("sito.db"), FakeBin::new(bin.path()));
        let older = id_of(&scan, OLDER);
        let newer = id_of(&scan, NEWER);
        reject_in(&mut scan, &mut actions, older, first).unwrap();
        reject_in(&mut scan, &mut actions, newer, second).unwrap();
        let place_of = |target, name| match target {
            RejectTarget::Folder(folder) => dir.path().join(folder.as_str()).join(name),
            RejectTarget::Trash => bin.path().join(name),
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
    fn undo_restores_the_rejection_and_leaves_the_accepts_around_it() {
        let dir = gallery();
        let mut scan = scan(dir.path());
        let mut actions = started(&dir.path().join("sito.db"));
        let id = id_of(&scan, NEWER);
        set_mark(&scan, &actions, NEWER, AcceptChange::Accepted);
        reject_in(&mut scan, &mut actions, id, FOLDER).unwrap();
        set_mark(&scan, &actions, OLDER, AcceptChange::Accepted);
        assert_eq!(accepted_names(&scan, &actions), [OLDER]);

        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();

        assert_eq!(
            undone,
            RestoredDto {
                id: Some(id),
                name: NEWER.into(),
                restored_name: None,
            }
        );
        // Back from "Odrzucone", still accepted, and the newer accept stays.
        assert_eq!(accepted_names(&scan, &actions), [NEWER, OLDER]);
        assert_eq!(undo_in(&mut scan, &actions).unwrap(), None);
        assert_eq!(accepted_names(&scan, &actions), [NEWER, OLDER]);
    }

    #[test]
    fn undo_with_a_taken_name_restores_under_a_free_one_with_its_mark() {
        let dir = gallery();
        let mut scan = scan(dir.path());
        let mut actions = started(&dir.path().join("sito.db"));
        let id = id_of(&scan, NEWER);
        set_mark(&scan, &actions, NEWER, AcceptChange::Accepted);
        reject_in(&mut scan, &mut actions, id, FOLDER).unwrap();
        fs::write(dir.path().join(NEWER), "another file").unwrap();

        let undone = undo_in(&mut scan, &actions).unwrap().unwrap();

        let renamed = "pixel_2024-06-01_12-00-05 (2).jpg";
        assert_eq!(undone.restored_name.as_deref(), Some(renamed));
        assert_eq!(undone.name, NEWER);
        assert_eq!(
            file_name(&scan.photo(undone.id.unwrap()).unwrap().path),
            renamed
        );
        assert_eq!(fs::read(dir.path().join(NEWER)).unwrap(), b"another file");
        assert_eq!(accepted_names(&scan, &actions), [renamed]);
        assert_eq!(undo_in(&mut scan, &actions).unwrap(), None);
    }

    #[test]
    fn rejecting_a_vanished_photo_logs_nothing() {
        let dir = gallery();
        let mut scan = scan(dir.path());
        let mut actions = started(&dir.path().join("sito.db"));
        let id = id_of(&scan, NEWER);
        fs::rename(dir.path().join(NEWER), dir.path().join("renamed.jpg")).unwrap();

        for destination in [FOLDER, RejectTarget::Trash] {
            assert_eq!(
                reject_in(&mut scan, &mut actions, id, destination),
                Err(CommandError::PhotoMissing)
            );
        }

        assert!(scan.photo(id).is_some());
        assert_eq!(undo_state(&actions, dir.path()), UndoStateDto::Nothing);
        assert!(!dir.path().join("Odrzucone").exists());
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
        reject_both_ways_and_undo(FOLDER, RejectTarget::Trash);
    }

    #[test]
    fn undoes_a_folder_rejection_and_then_a_trash_one() {
        reject_both_ways_and_undo(RejectTarget::Trash, FOLDER);
    }
}
