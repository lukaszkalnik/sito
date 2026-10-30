//! Which files belong to a photo, so they leave and return together.

use std::path::{Path, PathBuf};

use crate::scan::find_companion;
use crate::takeout::find_own_sidecar;

/// The files that belong to one photo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoFiles {
    /// The photo itself.
    pub photo: PathBuf,
    /// Its own Google Takeout date file (not one shared with other photos).
    pub sidecar: Option<PathBuf>,
    /// The RAW file of a RAW+JPEG pair.
    pub companion: Option<Companion>,
}

/// The RAW file paired with a JPEG photo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Companion {
    /// The RAW file.
    pub path: PathBuf,
    /// Its own Google Takeout date file.
    pub sidecar: Option<PathBuf>,
}

impl PhotoFiles {
    /// Finds the files that belong to the photo at `path`, looking at the
    /// other files in its folder.
    #[must_use]
    pub fn find(path: &Path) -> Self {
        let companion = find_companion(path).map(|file| Companion {
            sidecar: find_own_sidecar(&file.path),
            path: file.path,
        });
        Self {
            photo: path.to_path_buf(),
            sidecar: find_own_sidecar(path),
            companion,
        }
    }

    /// All the files, the photo first, then its date file, the RAW file and
    /// the RAW file's date file.
    #[must_use]
    pub fn all(&self) -> Vec<PathBuf> {
        let companion = self.companion.iter().flat_map(|companion| {
            std::iter::once(companion.path.clone()).chain(companion.sidecar.clone())
        });
        std::iter::once(self.photo.clone())
            .chain(self.sidecar.clone())
            .chain(companion)
            .collect()
    }
}
