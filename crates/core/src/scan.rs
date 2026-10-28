//! Recursive search of a folder tree for photos.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use walkdir::{DirEntry, WalkDir};

use crate::raw::RawFormat;
use crate::takeout::{is_sidecar_candidate, pair_sidecars};

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

/// Size and modification time of a file, to tell whether it changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStamp {
    /// File size in bytes.
    pub size: u64,
    /// Last modification time, if the file system reports one.
    pub modified: Option<SystemTime>,
}

/// A file with its size and modification time, e.g. a photo's Takeout date
/// file or the RAW file paired with a JPEG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StampedFile {
    /// Full path of the file.
    pub path: PathBuf,
    /// Its size and modification time.
    pub stamp: FileStamp,
}

impl StampedFile {
    /// The file at `path` with its current stamp, if it can be inspected.
    #[must_use]
    pub fn read(path: &Path) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        Some(Self {
            path: path.to_path_buf(),
            stamp: FileStamp {
                size: meta.len(),
                modified: meta.modified().ok(),
            },
        })
    }
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
    /// The photo's Google Takeout date file, if its folder has one.
    pub sidecar: Option<StampedFile>,
    /// The RAW file of a RAW+JPEG pair, when this is the pair's JPEG.
    pub companion: Option<StampedFile>,
}

/// Result of a finished search.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchResult {
    /// Photos found, in walk order (without rejected ones).
    pub photos: Vec<FoundFile>,
    /// Photos inside [`REJECTED_DIR`] folders, in walk order. They are not
    /// shown, but series are found including them, so a series keeps its
    /// shape after its weaker shots were rejected.
    pub rejected: Vec<FoundFile>,
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

/// The kind of a photo file, which decides how it is read and shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhotoKind {
    /// A JPEG (`.jpg`, `.jpeg`).
    Jpeg,
    /// A camera RAW file.
    Raw(RawFormat),
}

/// The kind of the photo at `path`, by its extension (any letter case), or
/// `None` if it is not a photo Sito reads.
#[must_use]
pub fn photo_kind(path: &Path) -> Option<PhotoKind> {
    let ext = path.extension()?.to_str()?;
    if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") {
        return Some(PhotoKind::Jpeg);
    }
    RawFormat::from_path(path).map(PhotoKind::Raw)
}

/// Returns `true` if the path has the extension of a photo Sito reads
/// (JPEG or a RAW format, any letter case).
#[must_use]
pub fn is_photo(path: &Path) -> bool {
    photo_kind(path).is_some()
}

/// Recursively finds the photos under `root`.
///
/// Hidden folders (name starting with `.`) and folders containing a `.nomedia`
/// file are skipped. Photos inside [`REJECTED_DIR`] folders (in any letter
/// case, at any depth below `root`) go to [`SearchResult::rejected`] instead
/// of [`SearchResult::photos`]. Symbolic links are not followed. Entries that
/// cannot be read are counted in [`SearchResult::unreadable_entries`] and do
/// not abort the search.
///
/// Google Takeout date files next to the photos are attached to their photos
/// ([`FoundFile::sidecar`]).
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
    let mut sidecars = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| entry.depth() == 0 || !is_skipped_dir(entry));

    for entry in walker {
        result.entries_seen += 1;
        match entry {
            Ok(entry) if entry.file_type().is_file() && is_photo(entry.path()) => {
                match entry.metadata() {
                    Ok(meta) => {
                        let list = if is_in_rejected_dir(entry.path(), root) {
                            &mut result.rejected
                        } else {
                            &mut result.photos
                        };
                        list.push(FoundFile {
                            modified: meta.modified().ok(),
                            size: meta.len(),
                            path: entry.into_path(),
                            sidecar: None,
                            companion: None,
                        });
                    }
                    Err(_) => result.unreadable_entries += 1,
                }
            }
            Ok(entry) if entry.file_type().is_file() && is_sidecar_candidate(entry.path()) => {
                // A date file that cannot be inspected only means its photo
                // keeps its own date.
                if let Ok(meta) = entry.metadata() {
                    sidecars.push(StampedFile {
                        stamp: FileStamp {
                            size: meta.len(),
                            modified: meta.modified().ok(),
                        },
                        path: entry.into_path(),
                    });
                }
            }
            Ok(_) => {}
            Err(_) => result.unreadable_entries += 1,
        }
        if result.entries_seen % PROGRESS_INTERVAL == 0 {
            on_progress(progress_of(&result));
        }
    }

    group_files(&mut result, sidecars);
    on_progress(progress_of(&result));
    Ok(result)
}

/// Attaches Takeout date files to their photos and joins RAW+JPEG pairs.
fn group_files(result: &mut SearchResult, sidecars: Vec<StampedFile>) {
    attach_sidecars(&mut result.photos, &mut result.rejected, sidecars);
    result.photos = pair_raw_with_jpeg(std::mem::take(&mut result.photos));
    result.rejected = pair_raw_with_jpeg(std::mem::take(&mut result.rejected));
}

/// Joins a RAW file and a JPEG of the same name in the same folder (a camera
/// set to RAW+JPEG) into one photo: the JPEG, with the RAW file as its
/// companion. Without its own Takeout date file, the pair uses the RAW
/// file's. Other files keep their order.
fn pair_raw_with_jpeg(files: Vec<FoundFile>) -> Vec<FoundFile> {
    let key = |file: &FoundFile| {
        let stem = file.path.file_stem()?.to_str()?.to_lowercase();
        Some((file.path.parent()?.to_path_buf(), stem))
    };
    let mut jpeg_at: HashMap<(PathBuf, String), usize> = HashMap::new();
    for (index, file) in files.iter().enumerate() {
        if photo_kind(&file.path) == Some(PhotoKind::Jpeg)
            && let Some(key) = key(file)
        {
            jpeg_at.entry(key).or_insert(index);
        }
    }
    let mut paired: HashMap<usize, FoundFile> = HashMap::new();
    let mut kept = Vec::with_capacity(files.len());
    for (index, file) in files.into_iter().enumerate() {
        let jpeg = matches!(photo_kind(&file.path), Some(PhotoKind::Raw(_)))
            .then(|| key(&file).and_then(|key| jpeg_at.get(&key).copied()))
            .flatten()
            .filter(|jpeg| !paired.contains_key(jpeg));
        match jpeg {
            Some(jpeg) => {
                paired.insert(jpeg, file);
            }
            None => kept.push((index, file)),
        }
    }
    kept.into_iter()
        .map(|(index, mut file)| {
            if let Some(raw) = paired.remove(&index) {
                file.sidecar = file.sidecar.or(raw.sidecar);
                file.companion = Some(StampedFile {
                    path: raw.path,
                    stamp: FileStamp {
                        size: raw.size,
                        modified: raw.modified,
                    },
                });
            }
            file
        })
        .collect()
}

/// The RAW file paired with the JPEG at `path`: a RAW file of the same name
/// in the same folder.
#[must_use]
pub fn find_companion(path: &Path) -> Option<StampedFile> {
    if photo_kind(path) != Some(PhotoKind::Jpeg) {
        return None;
    }
    let stem = path.file_stem()?.to_str()?.to_lowercase();
    std::fs::read_dir(path.parent()?)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|file| matches!(photo_kind(file), Some(PhotoKind::Raw(_))))
        .find(|file| {
            file.file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.to_lowercase() == stem)
        })
        .and_then(|file| StampedFile::read(&file))
}

/// Gives each photo the Takeout date file of its folder that belongs to it.
fn attach_sidecars(
    photos: &mut [FoundFile],
    rejected: &mut [FoundFile],
    sidecars: Vec<StampedFile>,
) {
    let mut sidecars_by_folder: HashMap<PathBuf, Vec<StampedFile>> = HashMap::new();
    for sidecar in sidecars {
        if let Some(folder) = sidecar.path.parent() {
            sidecars_by_folder
                .entry(folder.to_path_buf())
                .or_default()
                .push(sidecar);
        }
    }
    if sidecars_by_folder.is_empty() {
        return;
    }
    let mut photos_by_folder: HashMap<PathBuf, Vec<&mut FoundFile>> = HashMap::new();
    for photo in photos.iter_mut().chain(rejected.iter_mut()) {
        if let Some(folder) = photo
            .path
            .parent()
            .filter(|f| sidecars_by_folder.contains_key(*f))
        {
            photos_by_folder
                .entry(folder.to_path_buf())
                .or_default()
                .push(photo);
        }
    }
    for (folder, mut folder_photos) in photos_by_folder {
        let Some(folder_sidecars) = sidecars_by_folder.get(&folder) else {
            continue;
        };
        let photo_paths: Vec<&Path> = folder_photos.iter().map(|p| p.path.as_path()).collect();
        let sidecar_paths: Vec<&Path> = folder_sidecars.iter().map(|s| s.path.as_path()).collect();
        for (photo, sidecar) in pair_sidecars(&photo_paths, &sidecar_paths) {
            folder_photos[photo].sidecar = Some(folder_sidecars[sidecar].clone());
        }
    }
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

/// Whether a folder between `root` and the file at `path` is a [`REJECTED_DIR`].
pub(crate) fn is_in_rejected_dir(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(Path::parent)
        .is_some_and(|dirs| {
            dirs.components().any(|dir| {
                dir.as_os_str()
                    .to_str()
                    .is_some_and(|name| name.eq_ignore_ascii_case(REJECTED_DIR))
            })
        })
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
            assert_eq!(photo_kind(Path::new(name)), Some(PhotoKind::Jpeg));
        }
    }

    #[test]
    fn recognizes_raw_extensions() {
        for name in [
            "P1010001.ORF",
            "a.cr2",
            "b.NEF",
            "c.arw",
            "d.rw2",
            "e.raf",
            "f.dng",
        ] {
            assert!(
                matches!(photo_kind(Path::new(name)), Some(PhotoKind::Raw(_))),
                "{name} should be a RAW photo"
            );
        }
    }

    #[test]
    fn rejects_other_extensions() {
        for name in ["d.png", "e.heic", "f.cr3", "noext", ".jpg", "g.jpg.txt"] {
            assert!(!is_photo(Path::new(name)), "{name} should not be a photo");
        }
    }
}
