//! Moving the cache to another folder: by renaming on the same drive, or by
//! copying and verifying every file on another one.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use thiserror::Error;

use super::{NO_MEDIA_FILE, PHOTO_CACHE_FILE, THUMBNAILS_DIR, thumbnails_dir};
use crate::thumbnail::thumbnail_files;

/// How a cache was moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcome {
    /// Renamed on the same drive: the old folder no longer holds the cache.
    Renamed,
    /// Copied and verified: the old copy is still there, for the caller to
    /// remove with [`super::remove_cache`] once it uses the new one.
    Copied,
}

/// How far a copy has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveProgress {
    /// Files copied and verified so far.
    pub done: u64,
    /// Files to copy in total.
    pub total: u64,
}

/// Why a cache could not be moved. The old cache is then left as it was,
/// and what was copied of it is removed again.
#[derive(Debug, Error)]
pub enum MoveError {
    /// The target folder cannot be created or written.
    #[error("cannot write to {path}")]
    NotWritable {
        /// The target folder.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
    /// A file could not be read, copied or renamed.
    #[error("cannot move {path}")]
    Io {
        /// The file that failed.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
    /// A copy differs from its original.
    #[error("the copy of {0} differs from it")]
    Mismatch(PathBuf),
    /// `should_stop` said so before the copy was complete.
    #[error("the move was stopped")]
    Stopped,
}

/// How the files are moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Strategy {
    /// Rename where possible, copy otherwise.
    Auto,
    /// Always copy, as across drives.
    #[cfg(test)]
    Copy,
}

/// Copies one file, returning the bytes written.
type CopyFile = fn(&Path, &Path) -> io::Result<u64>;

/// Copies the file `from` to `to`, replacing a file there.
fn plain_copy(from: &Path, to: &Path) -> io::Result<u64> {
    fs::copy(from, to)
}

/// Moves the cache in folder `from` (its thumbnails and photo cache, see
/// [`super`]) into folder `to`, creating it and putting a
/// [`NO_MEDIA_FILE`] into it. The photo cache must be closed and nothing
/// may write thumbnails meanwhile.
///
/// On the same drive (and with no thumbnail folder in `to` yet) the folder
/// is renamed, which is instant. Otherwise every file is copied and checked
/// (size and BLAKE3 hash) against its original, reporting progress after
/// each one; `should_stop` is asked before each file. Files of the same name
/// in `to` are replaced (a cache left there before); other files there stay.
///
/// # Errors
///
/// Returns a [`MoveError`]; the cache in `from` is then unchanged and the
/// files copied so far are removed from `to`.
pub fn move_cache(
    from: &Path,
    to: &Path,
    should_stop: impl Fn() -> bool,
    on_progress: impl FnMut(MoveProgress),
) -> Result<MoveOutcome, MoveError> {
    move_with(
        from,
        to,
        Strategy::Auto,
        plain_copy,
        &should_stop,
        on_progress,
    )
}

fn move_with(
    from: &Path,
    to: &Path,
    strategy: Strategy,
    copy_file: CopyFile,
    should_stop: &dyn Fn() -> bool,
    on_progress: impl FnMut(MoveProgress),
) -> Result<MoveOutcome, MoveError> {
    let created = prepare_target(to)?;
    if strategy == Strategy::Auto && rename_cache(from, to).is_ok() {
        return Ok(MoveOutcome::Renamed);
    }
    let copy = CacheCopy {
        from,
        to,
        copy_file,
        should_stop,
    };
    copy.run(on_progress).inspect_err(|_| {
        copy.undo();
        if created.no_media {
            let _ = fs::remove_file(to.join(NO_MEDIA_FILE));
        }
        if created.dir {
            // Stays if anything else is in it.
            let _ = fs::remove_dir(to);
        }
    })?;
    Ok(MoveOutcome::Copied)
}

/// What [`prepare_target`] created.
#[derive(Debug, Clone, Copy)]
struct Created {
    /// The target folder.
    dir: bool,
    /// Its [`NO_MEDIA_FILE`].
    no_media: bool,
}

/// Creates the folder `to` and its [`NO_MEDIA_FILE`] where missing.
fn prepare_target(to: &Path) -> Result<Created, MoveError> {
    let not_writable = |source| MoveError::NotWritable {
        path: to.to_path_buf(),
        source,
    };
    let dir = !to.is_dir();
    fs::create_dir_all(to).map_err(not_writable)?;
    let path = to.join(NO_MEDIA_FILE);
    let no_media = !path.exists();
    if no_media {
        fs::write(&path, b"").map_err(not_writable)?;
    }
    Ok(Created { dir, no_media })
}

/// Renames the thumbnail folder and the photo cache from `from` into `to`,
/// or leaves both where they were. Fails across drives, and when `to`
/// already has a thumbnail folder (which a rename would not merge).
fn rename_cache(from: &Path, to: &Path) -> io::Result<()> {
    let (old_thumbs, new_thumbs) = (thumbnails_dir(from), thumbnails_dir(to));
    if new_thumbs.symlink_metadata().is_ok() {
        return Err(io::Error::from(io::ErrorKind::AlreadyExists));
    }
    let has_thumbs = old_thumbs.is_dir();
    if has_thumbs {
        fs::rename(&old_thumbs, &new_thumbs)?;
    }
    let old_db = from.join(PHOTO_CACHE_FILE);
    if !old_db.exists() {
        return Ok(());
    }
    fs::rename(&old_db, to.join(PHOTO_CACHE_FILE)).inspect_err(|_| {
        if has_thumbs {
            // Back where it was, so the copy starts from an unchanged cache.
            let _ = fs::rename(&new_thumbs, &old_thumbs);
        }
    })
}

/// A copy of the cache from one folder to another.
struct CacheCopy<'a> {
    from: &'a Path,
    to: &'a Path,
    copy_file: CopyFile,
    should_stop: &'a dyn Fn() -> bool,
}

impl CacheCopy<'_> {
    /// The cache's files, relative to its folder.
    fn files(&self) -> Result<Vec<PathBuf>, MoveError> {
        let thumbnails = thumbnails_dir(self.from);
        let mut files: Vec<PathBuf> = thumbnail_files(&thumbnails)
            .map_err(|source| MoveError::Io {
                path: thumbnails.clone(),
                source,
            })?
            .into_iter()
            .filter_map(|file| {
                file.path
                    .strip_prefix(self.from)
                    .ok()
                    .map(Path::to_path_buf)
            })
            .collect();
        if self.from.join(PHOTO_CACHE_FILE).is_file() {
            files.push(PathBuf::from(PHOTO_CACHE_FILE));
        }
        Ok(files)
    }

    /// Copies and verifies every file.
    fn run(&self, mut on_progress: impl FnMut(MoveProgress)) -> Result<(), MoveError> {
        let files = self.files()?;
        let total = files.len() as u64;
        for (done, file) in (1..).zip(&files) {
            if (self.should_stop)() {
                return Err(MoveError::Stopped);
            }
            self.copy_one(file)?;
            on_progress(MoveProgress { done, total });
        }
        Ok(())
    }

    /// Copies the file `relative` and checks the copy against it.
    fn copy_one(&self, relative: &Path) -> Result<(), MoveError> {
        let (source, target) = (self.from.join(relative), self.to.join(relative));
        let io_error = |source| MoveError::Io {
            path: relative.to_path_buf(),
            source,
        };
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir).map_err(io_error)?;
        }
        (self.copy_file)(&source, &target).map_err(io_error)?;
        let original = fingerprint(&source).map_err(io_error)?;
        if fingerprint(&target).map_err(io_error)? != original {
            return Err(MoveError::Mismatch(relative.to_path_buf()));
        }
        Ok(())
    }

    /// Removes what was copied into the target folder. Same-named files that
    /// were there before are gone too: they were replaced, and are cache.
    fn undo(&self) {
        // Removing what the target holds of the cache is exactly that: the
        // files listed in the source, plus the folders they leave empty.
        let Ok(files) = self.files() else {
            return;
        };
        for file in &files {
            let _ = fs::remove_file(self.to.join(file));
        }
        let thumbnails = self.to.join(THUMBNAILS_DIR);
        if let Ok(entries) = fs::read_dir(&thumbnails) {
            for entry in entries.flatten() {
                let _ = fs::remove_dir(entry.path());
            }
        }
        let _ = fs::remove_dir(&thumbnails);
    }
}

/// The size and BLAKE3 hash of the file at `path`.
fn fingerprint(path: &Path) -> io::Result<(u64, blake3::Hash)> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0; 64 * 1024];
    let mut len: u64 = 0;
    loop {
        let read = file.read(&mut buffer)?;
        let Some(chunk) = buffer.get(..read).filter(|chunk| !chunk.is_empty()) else {
            return Ok((len, hasher.finalize()));
        };
        hasher.update(chunk);
        len = len.saturating_add(read as u64);
    }
}

#[cfg(test)]
mod tests;
