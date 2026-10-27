//! Recursive search of a folder tree for photos.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use walkdir::{DirEntry, WalkDir};

/// File name that marks an Android folder as "not gallery media".
const NOMEDIA_MARKER: &str = ".nomedia";

/// Name of the folder rejected photos are moved into, next to where they were.
/// Scans skip it (in any letter case), so rejected photos leave the timeline.
pub const REJECTED_DIR: &str = "Odrzucone";

/// Number of walked entries between two progress reports.
const PROGRESS_INTERVAL: u64 = 500;

/// Intermediate state reported while searching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SearchProgress {
    /// Files and folders visited so far.
    pub entries_seen: u64,
    /// Photos found so far.
    pub photos_found: u64,
}

/// A photo file found by [`find_photos`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundFile {
    /// Full path of the file.
    pub path: PathBuf,
    /// File size in bytes.
    pub size: u64,
    /// Last modification time, if the file system reports one.
    pub modified: Option<SystemTime>,
}

/// Result of a finished search.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchResult {
    /// Photos found, in walk order.
    pub photos: Vec<FoundFile>,
    /// Files and folders visited.
    pub entries_seen: u64,
    /// Entries that could not be read (e.g. permission denied).
    pub unreadable_entries: u64,
}

/// Errors that abort a scan.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// The scan root does not exist.
    #[error("folder not found: {0}")]
    NotFound(PathBuf),
    /// The scan root exists but is not a folder.
    #[error("not a folder: {0}")]
    NotADirectory(PathBuf),
    /// The scan root could not be inspected.
    #[error("cannot read {path}")]
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Returns `true` if the path has a photo extension (`.jpg` or `.jpeg`, any case).
#[must_use]
pub fn is_photo(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg"))
}

/// Recursively finds the photos under `root`.
///
/// Hidden folders (name starting with `.`), folders containing a `.nomedia`
/// file and [`REJECTED_DIR`] folders are skipped. Symbolic links are not
/// followed. Entries that cannot be
/// read are counted in [`SearchResult::unreadable_entries`] and do not abort
/// the search.
///
/// `on_progress` is called every few hundred entries and once at the end.
///
/// # Errors
///
/// Returns an error if `root` does not exist, is not a folder, or cannot be
/// inspected.
pub fn find_photos(
    root: &Path,
    mut on_progress: impl FnMut(SearchProgress),
) -> Result<SearchResult, ScanError> {
    check_root(root)?;

    let mut result = SearchResult::default();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| entry.depth() == 0 || !is_skipped_dir(entry));

    for entry in walker {
        result.entries_seen += 1;
        match entry {
            Ok(entry) if entry.file_type().is_file() && is_photo(entry.path()) => {
                match entry.metadata() {
                    Ok(meta) => result.photos.push(FoundFile {
                        modified: meta.modified().ok(),
                        size: meta.len(),
                        path: entry.into_path(),
                    }),
                    Err(_) => result.unreadable_entries += 1,
                }
            }
            Ok(_) => {}
            Err(_) => result.unreadable_entries += 1,
        }
        if result.entries_seen % PROGRESS_INTERVAL == 0 {
            on_progress(progress_of(&result));
        }
    }

    on_progress(progress_of(&result));
    Ok(result)
}

fn check_root(root: &Path) -> Result<(), ScanError> {
    match root.metadata() {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(ScanError::NotADirectory(root.to_path_buf())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Err(ScanError::NotFound(root.to_path_buf()))
        }
        Err(source) => Err(ScanError::Io {
            path: root.to_path_buf(),
            source,
        }),
    }
}

fn is_skipped_dir(entry: &DirEntry) -> bool {
    entry.file_type().is_dir()
        && (is_hidden(entry) || is_rejected(entry) || entry.path().join(NOMEDIA_MARKER).exists())
}

fn is_rejected(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| name.eq_ignore_ascii_case(REJECTED_DIR))
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| name.starts_with('.'))
}

fn progress_of(result: &SearchResult) -> SearchProgress {
    SearchProgress {
        entries_seen: result.entries_seen,
        photos_found: result.photos.len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_jpeg_extensions_case_insensitively() {
        for name in ["a.jpg", "B.JPEG", "c.Jpg", "dir/d.jpeg"] {
            assert!(is_photo(Path::new(name)), "{name} should be a photo");
        }
    }

    #[test]
    fn rejects_other_extensions() {
        for name in ["d.png", "e.heic", "f.dng", "noext", ".jpg", "g.jpg.txt"] {
            assert!(!is_photo(Path::new(name)), "{name} should not be a photo");
        }
    }
}
