//! Moving rejected photos into the [`REJECTED_DIR`] subfolder of their own
//! folder, and back, with every file that belongs to the photo (its Takeout
//! date file, the RAW file of a RAW+JPEG pair and that file's date file).
//! Files are only ever moved, never deleted or overwritten.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::scan::REJECTED_DIR;

mod files;

pub use files::{Companion, PhotoFiles};

/// Name ending given to a date file moved along with its photo, so it pairs
/// with the photo by name wherever the photo is renamed to.
const SIDECAR_SUFFIX: &str = ".supplemental-metadata.json";

/// Error while moving a photo into or out of [`REJECTED_DIR`].
#[derive(Debug, Error)]
pub enum RejectError {
    /// The photo is not inside a folder.
    #[error("{0} has no parent folder")]
    NoParent(PathBuf),
    /// The file to move back is no longer in the rejected folder.
    #[error("{0} no longer exists")]
    Missing(PathBuf),
    /// Another file now has the original name, so moving back would overwrite it.
    #[error("{0} already exists")]
    Occupied(PathBuf),
    /// The file system refused the move.
    #[error("cannot move {from} to {to}")]
    Io {
        /// The file that was to be moved.
        from: PathBuf,
        /// Where it was to be moved.
        to: PathBuf,
        /// The underlying file system error.
        #[source]
        source: std::io::Error,
    },
}

/// One file moved along with a rejected photo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMove {
    /// Where the file was before the photo was rejected.
    pub from: PathBuf,
    /// Where it is in the [`REJECTED_DIR`] folder.
    pub to: PathBuf,
}

/// A photo moved into the rejected folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    /// Where the photo is now.
    pub path: PathBuf,
    /// Whether the [`REJECTED_DIR`] folder was created for it (it did not
    /// exist before).
    pub created_dir: bool,
    /// The other files of the photo, moved along with it, in the order they
    /// are moved.
    pub extra: Vec<FileMove>,
}

/// What [`restore`] did besides moving the photo back.
#[derive(Debug, Default)]
pub struct Restored {
    /// Why some of the photo's other files could not be moved back; the
    /// photo is back in any case.
    pub errors: Vec<RejectError>,
}

/// Moves the photo at `path` into the [`REJECTED_DIR`] subfolder of its
/// folder: [`prepare`] followed by [`complete`].
///
/// # Errors
///
/// Returns an error if the folder cannot be created or a file cannot be moved.
pub fn reject(path: &Path) -> Result<Rejected, RejectError> {
    let rejected = prepare(path)?;
    complete(path, &rejected)?;
    Ok(rejected)
}

/// Creates the [`REJECTED_DIR`] subfolder of the folder of `path` if needed
/// and picks where the photo and its other files go, without moving anything
/// yet, so the move can be logged before it happens.
///
/// Files already there are kept: the photo is then named `<stem> (2).<ext>`,
/// `(3)` and so on, and the RAW file of a RAW+JPEG pair gets the same number
/// so the two stay a pair. Their own Google Takeout date files go along,
/// named after their new names (`<name>.supplemental-metadata.json`); a date
/// file shared with other photos (an edited copy's) stays.
///
/// # Errors
///
/// Returns an error if `path` has no folder or file name, or the folder cannot
/// be created.
pub fn prepare(path: &Path) -> Result<Rejected, RejectError> {
    let no_parent = || RejectError::NoParent(path.to_path_buf());
    let parent = path.parent().ok_or_else(no_parent)?;
    let name = path.file_name().ok_or_else(no_parent)?;
    let dir = parent.join(REJECTED_DIR);
    let created_dir = !dir.is_dir();
    fs::create_dir_all(&dir).map_err(|source| RejectError::Io {
        from: path.to_path_buf(),
        to: dir.clone(),
        source,
    })?;

    let files = PhotoFiles::find(path);
    let mut names = vec![name];
    names.extend(
        files
            .companion
            .as_ref()
            .and_then(|companion| companion.path.file_name()),
    );
    let targets = free_names(&dir, &names).ok_or_else(|| RejectError::Occupied(dir.join(name)))?;

    let mut extra = Vec::new();
    extra.extend(sidecar_move(files.sidecar, &targets[0], &dir));
    if let (Some(companion), Some(companion_target)) = (files.companion, targets.get(1)) {
        extra.extend(sidecar_move(companion.sidecar, companion_target, &dir));
        extra.push(FileMove {
            from: companion.path,
            to: companion_target.clone(),
        });
    }
    Ok(Rejected {
        path: targets[0].clone(),
        created_dir,
        extra,
    })
}

/// The move of the date file `sidecar` of a photo file that goes to `target`
/// in `dir`.
fn sidecar_move(sidecar: Option<PathBuf>, target: &Path, dir: &Path) -> Option<FileMove> {
    let from = sidecar?;
    let mut name = target.file_name()?.to_os_string();
    name.push(SIDECAR_SUFFIX);
    let to = free_names(dir, &[&name])?.pop()?;
    Some(FileMove { from, to })
}

/// Moves the photo at `path` and its other files to where [`prepare`] put
/// them. The other files go first and the photo last, so a failure never
/// leaves the photo moved without them; after a failure, every file already
/// moved goes back, and a [`REJECTED_DIR`] folder that `prepare` created is
/// removed again while empty.
///
/// # Errors
///
/// Returns [`RejectError::Occupied`] if a target exists by now, or an error
/// if a file cannot be moved.
pub fn complete(path: &Path, rejected: &Rejected) -> Result<(), RejectError> {
    let result = move_all(path, rejected);
    if result.is_err() {
        abandon(rejected);
    }
    result
}

fn move_all(path: &Path, rejected: &Rejected) -> Result<(), RejectError> {
    if rejected.path.exists() {
        return Err(RejectError::Occupied(rejected.path.clone()));
    }
    let mut moved: Vec<&FileMove> = Vec::new();
    let mut result = Ok(());
    for file in &rejected.extra {
        result = move_to_free(&file.from, &file.to);
        if result.is_err() {
            break;
        }
        moved.push(file);
    }
    if result.is_ok() {
        result = move_file(path, &rejected.path);
    }
    if result.is_err() {
        for file in moved.into_iter().rev() {
            // Best effort: a file left in the rejected folder is still there
            // to be found; nothing is lost.
            let _ = move_to_free(&file.to, &file.from);
        }
    }
    result
}

/// Undoes [`prepare`] for a photo that will not be moved: removes the
/// [`REJECTED_DIR`] folder again if `prepare` created it and it is still empty.
pub fn abandon(rejected: &Rejected) {
    if rejected.created_dir {
        remove_if_empty(rejected.path.parent());
    }
}

/// Moves a rejected photo from `from` back to `to`, its path before it was
/// rejected, and then its other files `extra` back to where they were. With
/// `remove_empty_dir`, the [`REJECTED_DIR`] folder it was in is removed if
/// that leaves it empty; a folder with other files stays.
///
/// # Errors
///
/// Returns [`RejectError::Missing`] if `from` is gone, [`RejectError::Occupied`]
/// if `to` exists again, or an error if the photo cannot be moved. Other
/// files that cannot be moved back are reported in [`Restored::errors`].
pub fn restore(
    from: &Path,
    to: &Path,
    extra: &[FileMove],
    remove_empty_dir: bool,
) -> Result<Restored, RejectError> {
    if !from.is_file() {
        return Err(RejectError::Missing(from.to_path_buf()));
    }
    if to.exists() {
        return Err(RejectError::Occupied(to.to_path_buf()));
    }
    move_file(from, to)?;
    let errors = extra
        .iter()
        .filter_map(|file| move_back(file).err())
        .collect();
    if remove_empty_dir {
        remove_if_empty(from.parent());
    }
    Ok(Restored { errors })
}

fn move_back(file: &FileMove) -> Result<(), RejectError> {
    if !file.to.is_file() {
        return Err(RejectError::Missing(file.to.clone()));
    }
    move_to_free(&file.to, &file.from)
}

/// Moves `from` to `to` unless `to` exists.
fn move_to_free(from: &Path, to: &Path) -> Result<(), RejectError> {
    if to.exists() {
        return Err(RejectError::Occupied(to.to_path_buf()));
    }
    move_file(from, to)
}

/// Removes the folder `dir` if it is empty; a folder with files stays.
fn remove_if_empty(dir: Option<&Path>) {
    if let Some(dir) = dir {
        // `remove_dir` only removes an empty folder; it never deletes files.
        let _ = fs::remove_dir(dir);
    }
}

/// Free paths in `dir` for files named `names`: the names themselves, or
/// with the first number that is free for all of them, `<stem> (n).<ext>`;
/// `None` if there is none. Non-Unicode names are kept as they are.
fn free_names(dir: &Path, names: &[&OsStr]) -> Option<Vec<PathBuf>> {
    let all_free = |paths: &Vec<PathBuf>| paths.iter().all(|path| !path.exists());
    let first: Vec<PathBuf> = names.iter().map(|name| dir.join(name)).collect();
    if all_free(&first) {
        return Some(first);
    }
    (2..=u32::MAX)
        .map(|n| {
            names
                .iter()
                .map(|name| dir.join(numbered_name(name, n)))
                .collect()
        })
        .find(all_free)
}

/// `<stem> (n).<ext>`, or `<stem> (n)` without an extension.
fn numbered_name(name: &OsStr, n: u32) -> OsString {
    let as_path = Path::new(name);
    let mut numbered = as_path.file_stem().unwrap_or(name).to_os_string();
    numbered.push(format!(" ({n})"));
    if let Some(ext) = as_path.extension() {
        numbered.push(".");
        numbered.push(ext);
    }
    numbered
}

fn move_file(from: &Path, to: &Path) -> Result<(), RejectError> {
    fs::rename(from, to).map_err(|source| RejectError::Io {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
        source,
    })
}
