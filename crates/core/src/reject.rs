//! Moving rejected photos into the [`REJECTED_DIR`] subfolder of their own
//! folder, and back. Files are only ever moved, never deleted or overwritten.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::scan::REJECTED_DIR;
use crate::takeout::find_sidecar;

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

/// A photo's Google Takeout date file, moved along with the photo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarMove {
    /// Where the date file was before the photo was rejected.
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
    /// The photo's Takeout date file, moved along with it.
    pub sidecar: Option<SidecarMove>,
}

/// What [`restore`] did besides moving the photo back.
#[derive(Debug, Default)]
pub struct Restored {
    /// Why the photo's date file could not be moved back, if it could not;
    /// the photo is back in any case.
    pub sidecar_error: Option<RejectError>,
}

/// Moves the photo at `path` into the [`REJECTED_DIR`] subfolder of its
/// folder: [`prepare`] followed by [`complete`].
///
/// # Errors
///
/// Returns an error if the folder cannot be created or the file cannot be moved.
pub fn reject(path: &Path) -> Result<Rejected, RejectError> {
    let rejected = prepare(path)?;
    complete(path, &rejected)?;
    Ok(rejected)
}

/// Creates the [`REJECTED_DIR`] subfolder of the folder of `path` if needed
/// and picks the photo's path in it, without moving anything yet, so the move
/// can be logged before it happens. A file already there with the same name
/// is kept: the photo is then named `<stem> (2).<ext>`, `(3)` and so on.
///
/// A Google Takeout date file of the photo goes along, named after the
/// photo's new name (`<name>.supplemental-metadata.json`).
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
    let target = free_name(&dir, name).ok_or_else(|| RejectError::Occupied(dir.join(name)))?;
    let sidecar = find_sidecar(path).and_then(|from| {
        let mut sidecar_name = target.file_name()?.to_os_string();
        sidecar_name.push(SIDECAR_SUFFIX);
        let to = free_name(&dir, &sidecar_name)?;
        Some(SidecarMove { from, to })
    });
    Ok(Rejected {
        path: target,
        created_dir,
        sidecar,
    })
}

/// Moves the photo at `path`, and its date file, to where [`prepare`] put
/// them. The date file goes first, so a failure never leaves the photo moved
/// without it; if the photo then cannot move, its date file is moved back.
/// After a failure, a [`REJECTED_DIR`] folder that `prepare` created is
/// removed again while empty.
///
/// # Errors
///
/// Returns [`RejectError::Occupied`] if a target exists by now, or an error
/// if a file cannot be moved.
pub fn complete(path: &Path, rejected: &Rejected) -> Result<(), RejectError> {
    let result = move_with_sidecar(path, rejected);
    if result.is_err() {
        abandon(rejected);
    }
    result
}

fn move_with_sidecar(path: &Path, rejected: &Rejected) -> Result<(), RejectError> {
    if rejected.path.exists() {
        return Err(RejectError::Occupied(rejected.path.clone()));
    }
    if let Some(sidecar) = &rejected.sidecar {
        move_to_free(&sidecar.from, &sidecar.to)?;
    }
    let moved = move_file(path, &rejected.path);
    if moved.is_err()
        && let Some(sidecar) = &rejected.sidecar
    {
        // Best effort: a date file left in the rejected folder only means the
        // photo shows its own date until it is found again.
        let _ = move_to_free(&sidecar.to, &sidecar.from);
    }
    moved
}

/// Undoes [`prepare`] for a photo that will not be moved: removes the
/// [`REJECTED_DIR`] folder again if `prepare` created it and it is still empty.
pub fn abandon(rejected: &Rejected) {
    if rejected.created_dir {
        remove_if_empty(rejected.path.parent());
    }
}

/// Moves a rejected photo from `from` back to `to`, its path before it was
/// rejected, and then its date file `sidecar` back to where it was. With
/// `remove_empty_dir`, the [`REJECTED_DIR`] folder it was in is removed if
/// that leaves it empty; a folder with other files stays.
///
/// # Errors
///
/// Returns [`RejectError::Missing`] if `from` is gone, [`RejectError::Occupied`]
/// if `to` exists again, or an error if the photo cannot be moved. A date
/// file that cannot be moved back is reported in [`Restored::sidecar_error`].
pub fn restore(
    from: &Path,
    to: &Path,
    sidecar: Option<&SidecarMove>,
    remove_empty_dir: bool,
) -> Result<Restored, RejectError> {
    if !from.is_file() {
        return Err(RejectError::Missing(from.to_path_buf()));
    }
    if to.exists() {
        return Err(RejectError::Occupied(to.to_path_buf()));
    }
    move_file(from, to)?;
    let sidecar_error = sidecar.and_then(|sidecar| move_back(sidecar).err());
    if remove_empty_dir {
        remove_if_empty(from.parent());
    }
    Ok(Restored { sidecar_error })
}

fn move_back(sidecar: &SidecarMove) -> Result<(), RejectError> {
    if !sidecar.to.is_file() {
        return Err(RejectError::Missing(sidecar.to.clone()));
    }
    move_to_free(&sidecar.to, &sidecar.from)
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

/// `dir/name`, or the first free `dir/<stem> (n).<ext>` if that exists;
/// `None` if no name is free. Non-Unicode names are kept as they are.
fn free_name(dir: &Path, name: &OsStr) -> Option<PathBuf> {
    let first = dir.join(name);
    if !first.exists() {
        return Some(first);
    }
    let as_path = Path::new(name);
    let stem = as_path.file_stem().unwrap_or(name);
    let ext = as_path.extension();
    (2..=u32::MAX)
        .map(|n| dir.join(numbered_name(stem, n, ext)))
        .find(|candidate| !candidate.exists())
}

/// `<stem> (n).<ext>`, or `<stem> (n)` without an extension.
fn numbered_name(stem: &OsStr, n: u32, ext: Option<&OsStr>) -> OsString {
    let mut name = stem.to_os_string();
    name.push(format!(" ({n})"));
    if let Some(ext) = ext {
        name.push(".");
        name.push(ext);
    }
    name
}

fn move_file(from: &Path, to: &Path) -> Result<(), RejectError> {
    fs::rename(from, to).map_err(|source| RejectError::Io {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
        source,
    })
}
