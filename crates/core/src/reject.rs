//! Moving rejected photos into the [`REJECTED_DIR`] subfolder of their own
//! folder, and back. Files are only ever moved, never deleted or overwritten.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::scan::REJECTED_DIR;

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

/// A photo moved into the rejected folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    /// Where the photo is now.
    pub path: PathBuf,
    /// Whether the [`REJECTED_DIR`] folder was created for it (it did not
    /// exist before).
    pub created_dir: bool,
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
    Ok(Rejected {
        path: target,
        created_dir,
    })
}

/// Moves the photo at `path` to where [`prepare`] put it. If that fails, a
/// [`REJECTED_DIR`] folder that `prepare` created is removed again while empty.
///
/// # Errors
///
/// Returns [`RejectError::Occupied`] if the target exists by now, or an error
/// if the file cannot be moved.
pub fn complete(path: &Path, rejected: &Rejected) -> Result<(), RejectError> {
    let result = if rejected.path.exists() {
        Err(RejectError::Occupied(rejected.path.clone()))
    } else {
        move_file(path, &rejected.path)
    };
    if result.is_err() {
        abandon(rejected);
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
/// rejected. With `remove_empty_dir`, the [`REJECTED_DIR`] folder it was in
/// is removed if that leaves it empty; a folder with other files stays.
///
/// # Errors
///
/// Returns [`RejectError::Missing`] if `from` is gone, [`RejectError::Occupied`]
/// if `to` exists again, or an error if the file cannot be moved.
pub fn restore(from: &Path, to: &Path, remove_empty_dir: bool) -> Result<(), RejectError> {
    if !from.is_file() {
        return Err(RejectError::Missing(from.to_path_buf()));
    }
    if to.exists() {
        return Err(RejectError::Occupied(to.to_path_buf()));
    }
    move_file(from, to)?;
    if remove_empty_dir {
        remove_if_empty(from.parent());
    }
    Ok(())
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
