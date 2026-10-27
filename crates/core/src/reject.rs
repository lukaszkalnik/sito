//! Moving rejected photos into the [`REJECTED_DIR`] subfolder of their own
//! folder, and back. Files are only ever moved, never deleted or overwritten.

use std::ffi::OsString;
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
        from: PathBuf,
        to: PathBuf,
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
/// folder, creating the folder if needed. A file already there with the same
/// name is kept: the photo is then named `<stem> (2).<ext>`, `(3)` and so on.
///
/// # Errors
///
/// Returns an error if the folder cannot be created or the file cannot be moved.
pub fn reject(path: &Path) -> Result<Rejected, RejectError> {
    let parent = path
        .parent()
        .ok_or_else(|| RejectError::NoParent(path.to_path_buf()))?;
    let name = path
        .file_name()
        .ok_or_else(|| RejectError::NoParent(path.to_path_buf()))?;
    let dir = parent.join(REJECTED_DIR);
    let created_dir = !dir.is_dir();
    fs::create_dir_all(&dir).map_err(|source| RejectError::Io {
        from: path.to_path_buf(),
        to: dir.clone(),
        source,
    })?;
    let target = free_name(&dir, name);
    move_file(path, &target)?;
    Ok(Rejected {
        path: target,
        created_dir,
    })
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
    if remove_empty_dir && let Some(dir) = from.parent() {
        // `remove_dir` only removes an empty folder; it never deletes files.
        let _ = fs::remove_dir(dir);
    }
    Ok(())
}

/// `dir/name`, or the first free `dir/<stem> (n).<ext>` if that exists.
fn free_name(dir: &Path, name: &std::ffi::OsStr) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let as_path = Path::new(name);
    let stem = as_path.file_stem().unwrap_or(name).to_string_lossy();
    let ext = as_path.extension().map(|e| e.to_string_lossy());
    (2..=u32::MAX)
        .map(|n| {
            let mut candidate = OsString::from(format!("{stem} ({n})"));
            if let Some(ext) = &ext {
                candidate.push(format!(".{ext}"));
            }
            dir.join(candidate)
        })
        .find(|candidate| !candidate.exists())
        .unwrap_or(first)
}

fn move_file(from: &Path, to: &Path) -> Result<(), RejectError> {
    fs::rename(from, to).map_err(|source| RejectError::Io {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
        source,
    })
}
