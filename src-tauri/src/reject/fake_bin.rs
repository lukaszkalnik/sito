//! A stand-in for the system trash in tests: a folder of its own.

use std::fs;
use std::path::{Path, PathBuf};

use sito_core::trash::{RestoredFromTrash, TrashError};

use super::trash::Bin;

/// Moves files into `dir` by name, and back.
#[derive(Debug)]
pub(super) struct FakeBin {
    /// Where trashed files are kept.
    pub dir: PathBuf,
    /// Whether the photos' drive has a trash.
    pub has_trash: bool,
    /// Whether files can be restored (unlike on macOS).
    pub can_restore: bool,
}

impl FakeBin {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            has_trash: true,
            can_restore: true,
        }
    }

    /// Where `file` is kept while in the trash.
    pub fn place_of(&self, file: &Path) -> PathBuf {
        self.dir.join(file.file_name().unwrap_or_default())
    }
}

impl Bin for FakeBin {
    fn has_trash(&self, _folder: &Path) -> bool {
        self.has_trash
    }

    fn put(&self, files: &[PathBuf]) -> Result<(), TrashError> {
        for file in files {
            fs::rename(file, self.place_of(file)).map_err(|err| TrashError::Trash {
                path: file.clone(),
                source: ::trash::into_unknown(err),
            })?;
        }
        Ok(())
    }

    fn take_back(&self, files: &[PathBuf], _at: i64) -> Result<RestoredFromTrash, TrashError> {
        if !self.can_restore {
            return Err(TrashError::RestoreUnsupported);
        }
        let mut errors = Vec::new();
        for (index, file) in files.iter().enumerate() {
            let result = take_back_one(&self.place_of(file), file);
            match result {
                Err(err) if index == 0 => return Err(err),
                Err(err) => errors.push(err),
                Ok(()) => {}
            }
        }
        Ok(RestoredFromTrash { errors })
    }
}

fn take_back_one(kept: &Path, file: &Path) -> Result<(), TrashError> {
    if !kept.exists() {
        return Err(TrashError::Missing(file.to_path_buf()));
    }
    if file.exists() {
        return Err(TrashError::Occupied(file.to_path_buf()));
    }
    fs::rename(kept, file).map_err(|err| TrashError::Restore(::trash::into_unknown(err)))
}
