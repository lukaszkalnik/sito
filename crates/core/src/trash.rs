//! Moving rejected photos into the system trash (the Recycle Bin on
//! Windows), and back. The trash keeps the files, so this is a move, not a
//! delete; it is only offered on drives that have a trash.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf, Prefix};

use ::trash::TrashItem;
use thiserror::Error;

/// How many seconds before the logged time of a move a file in the trash may
/// say it was deleted and still count as moved by it. The log is written just
/// before the move, but the two clocks round differently.
const CLOCK_SLACK_SECS: i64 = 2;

/// Error while moving files into the trash or back.
#[derive(Debug, Error)]
pub enum TrashError {
    /// The system refused to move the files into the trash.
    #[error("cannot move {path} to the trash")]
    Trash {
        /// The photo whose files were to be moved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: ::trash::Error,
    },
    /// The file is not in the trash (any more).
    #[error("{0} is not in the trash")]
    Missing(PathBuf),
    /// Another file now has the original name, so restoring would overwrite it.
    #[error("{0} already exists")]
    Occupied(PathBuf),
    /// The trash could not be read or the files could not be restored.
    #[error("cannot restore from the trash")]
    Restore(#[source] ::trash::Error),
    /// This system offers no way to restore files from the trash.
    #[error("restoring from the trash is not supported on this system")]
    RestoreUnsupported,
}

/// What [`restore_from_trash`] did besides restoring the photo.
#[derive(Debug, Default)]
pub struct RestoredFromTrash {
    /// Why some of the photo's other files were not restored; the photo is
    /// back in any case.
    pub errors: Vec<TrashError>,
}

/// Moves `files` (a photo first, then its other files) into the trash, in
/// one system operation.
///
/// # Errors
///
/// Returns [`TrashError::Trash`] if the system refuses.
pub fn move_to_trash(files: &[PathBuf]) -> Result<(), TrashError> {
    ::trash::delete_all(files).map_err(|source| TrashError::Trash {
        path: files.first().cloned().unwrap_or_default(),
        source,
    })
}

/// Moves `files` (a photo first, then its other files) back from the trash
/// to where they were, taking for each the newest file in the trash that was
/// deleted from that path at or after `trashed_at` (Unix seconds).
///
/// # Errors
///
/// Returns [`TrashError::Missing`] if the photo is not in the trash,
/// [`TrashError::Occupied`] if its original path is taken again, or
/// [`TrashError::Restore`] if the trash cannot be read or restored from.
/// Other files that are not restored are reported in
/// [`RestoredFromTrash::errors`].
#[cfg(not(target_os = "macos"))]
pub fn restore_from_trash(
    files: &[PathBuf],
    trashed_at: i64,
) -> Result<RestoredFromTrash, TrashError> {
    let items = ::trash::os_limited::list().map_err(TrashError::Restore)?;
    let picked = pick_items(items, files, trashed_at);
    let mut chosen = Vec::new();
    let mut errors = Vec::new();
    for (index, (file, item)) in files.iter().zip(picked).enumerate() {
        match restorable(file, item) {
            Ok(item) => chosen.push(item),
            // The photo itself must come back, or nothing does.
            Err(err) if index == 0 => return Err(err),
            Err(err) => errors.push(err),
        }
    }
    ::trash::os_limited::restore_all(chosen).map_err(TrashError::Restore)?;
    Ok(RestoredFromTrash { errors })
}

/// Restoring from the trash: not offered by the system's trash API here.
///
/// # Errors
///
/// Always [`TrashError::RestoreUnsupported`].
#[cfg(target_os = "macos")]
pub fn restore_from_trash(
    _files: &[PathBuf],
    _trashed_at: i64,
) -> Result<RestoredFromTrash, TrashError> {
    Err(TrashError::RestoreUnsupported)
}

/// The trash `item` of `file`, if it can go back without overwriting anything.
#[cfg(not(target_os = "macos"))]
fn restorable(file: &Path, item: Option<TrashItem>) -> Result<TrashItem, TrashError> {
    let item = item.ok_or_else(|| TrashError::Missing(file.to_path_buf()))?;
    if file.exists() {
        return Err(TrashError::Occupied(file.to_path_buf()));
    }
    Ok(item)
}

/// For each of `files`, the newest of `items` deleted from that path at or
/// after `trashed_at` (Unix seconds), if any. Older items from the same path
/// were deleted by something else and are left alone.
#[cfg_attr(
    all(target_os = "macos", not(test)),
    expect(dead_code, reason = "macOS has no restore from the trash yet")
)]
fn pick_items(items: Vec<TrashItem>, files: &[PathBuf], trashed_at: i64) -> Vec<Option<TrashItem>> {
    let mut picked: Vec<Option<TrashItem>> = vec![None; files.len()];
    let recent = items
        .into_iter()
        .filter(|item| item.time_deleted >= trashed_at - CLOCK_SLACK_SECS);
    for item in recent {
        let original = item.original_path();
        let Some(index) = files.iter().position(|file| same_path(file, &original)) else {
            continue;
        };
        let slot = &mut picked[index];
        if slot
            .as_ref()
            .is_none_or(|known| known.time_deleted < item.time_deleted)
        {
            *slot = Some(item);
        }
    }
    picked
}

/// Whether `a` and `b` name the same file, ignoring ASCII case (as the file
/// systems of Windows and macOS do) and whether a Windows drive is written
/// as `C:` or `\\?\C:` (the trash reports one, a canonical path the other).
fn same_path(a: &Path, b: &Path) -> bool {
    a.components().count() == b.components().count()
        && a.components()
            .zip(b.components())
            .all(|(a, b)| same_component(a, b))
}

fn same_component(a: Component<'_>, b: Component<'_>) -> bool {
    match (drive_letter(a), drive_letter(b)) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(&b),
        _ => OsStr::eq_ignore_ascii_case(a.as_os_str(), b.as_os_str()),
    }
}

fn drive_letter(component: Component<'_>) -> Option<u8> {
    let Component::Prefix(prefix) = component else {
        return None;
    };
    if let Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) = prefix.kind() {
        Some(letter)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    fn item(path: &str, time_deleted: i64) -> TrashItem {
        let path = Path::new(path);
        TrashItem {
            id: OsString::from(format!("{}#{time_deleted}", path.display())),
            name: path.file_name().unwrap().to_os_string(),
            original_parent: path.parent().unwrap().to_path_buf(),
            time_deleted,
        }
    }

    fn ids(picked: &[Option<TrashItem>]) -> Vec<Option<String>> {
        picked
            .iter()
            .map(|item| {
                item.as_ref()
                    .map(|item| item.id.to_string_lossy().into_owned())
            })
            .collect()
    }

    #[test]
    fn picks_the_newest_item_of_each_file_deleted_since_the_move() {
        let files = [
            PathBuf::from(r"C:\Zdjęcia\IMG_1.jpg"),
            PathBuf::from(r"C:\Zdjęcia\IMG_1.dng"),
        ];
        let items = vec![
            item(r"C:\Zdjęcia\IMG_1.jpg", 900),
            item(r"C:\Zdjęcia\IMG_1.jpg", 1000),
            item(r"C:\Zdjęcia\IMG_1.dng", 1001),
            item(r"C:\Zdjęcia\IMG_1.jpg", 1005),
            item(r"C:\Inne\IMG_1.jpg", 1010),
        ];

        let picked = pick_items(items, &files, 1000);

        assert_eq!(
            ids(&picked),
            [
                Some(r"C:\Zdjęcia\IMG_1.jpg#1005".to_owned()),
                Some(r"C:\Zdjęcia\IMG_1.dng#1001".to_owned()),
            ]
        );
    }

    #[test]
    fn leaves_out_items_deleted_before_the_move() {
        let files = [PathBuf::from(r"C:\Zdjęcia\IMG_1.jpg")];
        let items = vec![item(r"C:\Zdjęcia\IMG_1.jpg", 900)];

        assert_eq!(ids(&pick_items(items, &files, 1000)), [None]);
    }

    #[test]
    fn allows_for_clocks_that_round_differently() {
        let files = [PathBuf::from(r"C:\Zdjęcia\IMG_1.jpg")];
        let items = vec![item(r"C:\Zdjęcia\IMG_1.jpg", 999)];

        assert_eq!(
            ids(&pick_items(items, &files, 1000)),
            [Some(r"C:\Zdjęcia\IMG_1.jpg#999".to_owned())]
        );
    }

    #[cfg(windows)]
    #[test]
    fn matches_paths_regardless_of_case_and_verbatim_prefix() {
        assert!(same_path(
            Path::new(r"\\?\C:\Zdjęcia\IMG_1.JPG"),
            Path::new(r"c:\zdjęcia\img_1.jpg")
        ));
        assert!(!same_path(
            Path::new(r"C:\Zdjęcia\IMG_1.jpg"),
            Path::new(r"D:\Zdjęcia\IMG_1.jpg")
        ));
        assert!(!same_path(
            Path::new(r"C:\Zdjęcia\IMG_1.jpg"),
            Path::new(r"C:\Zdjęcia\Album\IMG_1.jpg")
        ));
    }

    /// Moves a temp file through the real trash and back. Not run by default,
    /// since it touches the user's Recycle Bin: `cargo test -- --ignored`.
    #[cfg(not(target_os = "macos"))]
    #[test]
    #[ignore = "uses the real system trash"]
    fn moves_a_file_into_the_real_trash_and_back() {
        let dir = tempfile::TempDir::new().unwrap();
        let photo = dir.path().join("sito-trash-test.jpg");
        std::fs::write(&photo, "photo").unwrap();
        let at = chrono::Utc::now().timestamp();

        move_to_trash(std::slice::from_ref(&photo)).unwrap();
        assert!(!photo.exists());
        restore_from_trash(std::slice::from_ref(&photo), at).unwrap();

        assert_eq!(std::fs::read_to_string(&photo).unwrap(), "photo");
    }
}
