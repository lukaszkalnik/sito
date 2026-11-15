//! Photos whose files vanished from the open folder (moved, renamed or
//! deleted outside Sito), or the open folder itself vanished: the protocols
//! answer `410 Gone` for them and tell the windows, at most once per
//! timeline revision, so the UI can offer a refresh.

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use sito_core::timeline::Photo;
use tauri::http::StatusCode;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::error::{CommandError, off_main_thread};
use crate::scan::state::AppState;

/// Event with the scan id when photo files of the open folder are gone.
pub const PHOTOS_MISSING: &str = "photos-missing";

/// Event with the scan id when the open folder itself is gone.
pub const FOLDER_MISSING: &str = "folder-missing";

/// What is gone when a photo's file cannot be found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Vanished {
    /// The photo's file; its folder is still there.
    Photo,
    /// The whole scanned folder (renamed, moved or its drive unplugged).
    Folder,
}

impl Vanished {
    /// What is gone of `photo` in the folder `root`; `None` if its file is
    /// there.
    pub fn of(photo: &Photo, root: &Path) -> Option<Self> {
        if photo.path.exists() {
            None
        } else if root.is_dir() {
            Some(Self::Photo)
        } else {
            Some(Self::Folder)
        }
    }

    fn event(self) -> &'static str {
        match self {
            Self::Photo => PHOTOS_MISSING,
            Self::Folder => FOLDER_MISSING,
        }
    }
}

/// The status a protocol answers for a photo whose file could not be read:
/// `410 Gone` if it vanished, `fallback` otherwise.
pub fn status(vanished: Option<Vanished>, fallback: StatusCode) -> StatusCode {
    vanished.map_or(fallback, |_| StatusCode::GONE)
}

/// Which vanished files were reported, so each kind is reported once per
/// scan and timeline revision however many photos fail.
#[derive(Debug, Default)]
pub struct Reported(Mutex<HashSet<(u64, u64, Vanished)>>);

impl Reported {
    /// Whether `vanished` is new for revision `revision` of scan `scan_id`;
    /// it then counts as reported.
    pub fn is_new(&self, scan_id: u64, revision: u64, vanished: Vanished) -> bool {
        let mut reported = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        // Only the newest revision matters; older entries are dropped.
        reported.retain(|&(scan, rev, _)| scan == scan_id && rev == revision);
        reported.insert((scan_id, revision, vanished))
    }
}

/// Checks after a failed read of photo `photo` of scan `scan_id` whether its
/// file or the folder vanished, and if so tells the windows (once per
/// revision). Returns what vanished.
pub fn check<R: Runtime>(app: &AppHandle<R>, scan_id: u64, photo: &Photo) -> Option<Vanished> {
    let state = app.state::<AppState>();
    let (root, revision) = {
        let scan = state.scan();
        let scan = scan.as_ref().filter(|scan| scan.id == scan_id)?;
        (scan.timeline.root.clone(), scan.revision())
    };
    let vanished = Vanished::of(photo, &root)?;
    if app.state::<Reported>().is_new(scan_id, revision, vanished) {
        announce(app, scan_id, vanished);
    }
    Some(vanished)
}

/// What is gone of photo `id` of the current scan: `null` if its file is
/// there (or there is no such photo), `"photo"` or `"folder"`. The viewer
/// asks after a photo failed to load, since an `<img>` does not tell why.
#[tauri::command]
pub async fn check_photo_file(id: usize, app: AppHandle) -> Result<Option<Vanished>, CommandError> {
    off_main_thread(move || {
        let state = app.state::<AppState>();
        let (photo, root) = {
            let scan = state.scan();
            let scan = scan.as_ref()?;
            (scan.photo(id)?.clone(), scan.timeline.root.clone())
        };
        Vanished::of(&photo, &root)
    })
    .await
}

/// Tells the windows that `vanished` is gone in scan `scan_id`.
pub fn announce<R: Runtime>(app: &AppHandle<R>, scan_id: u64, vanished: Vanished) {
    if let Err(err) = app.emit(vanished.event(), scan_id) {
        log::warn!("cannot announce {}: {err}", vanished.event());
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use sito_core::metadata::Orientation;
    use sito_core::timeline::{ExifInfo, TimeSource};

    use super::*;

    fn photo(path: PathBuf) -> Photo {
        Photo {
            path,
            size: 4,
            modified: None,
            taken_at: chrono::NaiveDateTime::default(),
            time_source: TimeSource::Exif,
            exif: ExifInfo::Read {
                orientation: Orientation::NORMAL,
                camera: None,
            },
            sidecar: None,
            companion: None,
        }
    }

    #[test]
    fn tells_a_vanished_photo_from_a_vanished_folder() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("2023");
        fs::create_dir(&root).unwrap();
        let there = root.join("IMG_1.jpg");
        fs::write(&there, b"photo").unwrap();

        assert_eq!(Vanished::of(&photo(there.clone()), &root), None);
        assert_eq!(
            Vanished::of(&photo(root.join("IMG_2.jpg")), &root),
            Some(Vanished::Photo)
        );

        fs::rename(&root, parent.path().join("Rok 2023")).unwrap();
        assert_eq!(Vanished::of(&photo(there), &root), Some(Vanished::Folder));
    }

    #[test]
    fn a_vanished_file_is_gone_and_others_keep_their_status() {
        assert_eq!(
            status(Some(Vanished::Photo), StatusCode::NOT_FOUND),
            StatusCode::GONE
        );
        assert_eq!(
            status(Some(Vanished::Folder), StatusCode::NOT_FOUND),
            StatusCode::GONE
        );
        assert_eq!(status(None, StatusCode::NOT_FOUND), StatusCode::NOT_FOUND);
    }

    #[test]
    fn each_kind_is_reported_once_per_revision() {
        let reported = Reported::default();

        assert!(reported.is_new(1, 0, Vanished::Photo));
        assert!(!reported.is_new(1, 0, Vanished::Photo));
        assert!(reported.is_new(1, 0, Vanished::Folder));
        assert!(!reported.is_new(1, 0, Vanished::Folder));
        // A new revision (e.g. after a refresh) or another scan reports again.
        assert!(reported.is_new(1, 1, Vanished::Photo));
        assert!(reported.is_new(2, 0, Vanished::Photo));
    }
}
