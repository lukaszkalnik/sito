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

/// Moves the photo at `path` into the [`REJECTED_DIR`] subfolder of its folder
/// and returns its new path. A file already there with the same name is kept:
/// the photo is then named `<stem> (2).<ext>`, `(3)` and so on.
///
/// # Errors
///
/// Returns an error if the folder cannot be created or the file cannot be moved.
pub fn reject(path: &Path) -> Result<PathBuf, RejectError> {
    let parent = path
        .parent()
        .ok_or_else(|| RejectError::NoParent(path.to_path_buf()))?;
    let name = path
        .file_name()
        .ok_or_else(|| RejectError::NoParent(path.to_path_buf()))?;
    let dir = parent.join(REJECTED_DIR);
    fs::create_dir_all(&dir).map_err(|source| RejectError::Io {
        from: path.to_path_buf(),
        to: dir.clone(),
        source,
    })?;
    let target = free_name(&dir, name);
    move_file(path, &target)?;
    Ok(target)
}

/// Moves a rejected photo from `from` back to `to`, its path before it was
/// rejected, and removes the [`REJECTED_DIR`] folder it was in if that is now
/// empty.
///
/// # Errors
///
/// Returns [`RejectError::Missing`] if `from` is gone, [`RejectError::Occupied`]
/// if `to` exists again, or an error if the file cannot be moved.
pub fn restore(from: &Path, to: &Path) -> Result<(), RejectError> {
    if !from.is_file() {
        return Err(RejectError::Missing(from.to_path_buf()));
    }
    if to.exists() {
        return Err(RejectError::Occupied(to.to_path_buf()));
    }
    move_file(from, to)?;
    if let Some(dir) = from.parent() {
        // Fails (and is left alone) while other files are still in the folder.
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
