//! Recursive scan of a folder tree for photos.

use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

/// File name that marks an Android folder as "not gallery media".
const NOMEDIA_MARKER: &str = ".nomedia";

/// Number of walked entries between two progress reports.
const PROGRESS_INTERVAL: u64 = 500;

/// Intermediate state reported while a scan is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScanProgress {
    /// Files and folders visited so far.
    pub entries_seen: u64,
    /// Photos found so far.
    pub photos_found: u64,
}

/// Result of a finished scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScanSummary {
    /// Number of photos found.
    pub photos_found: u64,
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

/// Recursively counts the photos under `root`.
///
/// Hidden folders (name starting with `.`) and folders containing a `.nomedia`
/// file are skipped. Symbolic links are not followed. Entries that cannot be
/// read are counted in [`ScanSummary::unreadable_entries`] and do not abort
/// the scan.
///
/// `on_progress` is called every few hundred entries and once at the end.
///
/// # Errors
///
/// Returns an error if `root` does not exist, is not a folder, or cannot be
/// inspected.
pub fn scan(
    root: &Path,
    mut on_progress: impl FnMut(ScanProgress),
) -> Result<ScanSummary, ScanError> {
    check_root(root)?;

    let mut summary = ScanSummary::default();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| entry.depth() == 0 || !is_skipped_dir(entry));

    for entry in walker {
        summary.entries_seen += 1;
        match entry {
            Ok(entry) if entry.file_type().is_file() && is_photo(entry.path()) => {
                summary.photos_found += 1;
            }
            Ok(_) => {}
            Err(_) => summary.unreadable_entries += 1,
        }
        if summary.entries_seen % PROGRESS_INTERVAL == 0 {
            on_progress(progress_of(&summary));
        }
    }

    on_progress(progress_of(&summary));
    Ok(summary)
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
    entry.file_type().is_dir() && (is_hidden(entry) || entry.path().join(NOMEDIA_MARKER).exists())
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| name.starts_with('.'))
}

fn progress_of(summary: &ScanSummary) -> ScanProgress {
    ScanProgress {
        entries_seen: summary.entries_seen,
        photos_found: summary.photos_found,
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
