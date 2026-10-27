//! All photos of a folder tree, with capture times, sorted newest first.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local, NaiveDateTime};
use rayon::prelude::*;

use crate::metadata::{Orientation, read_photo_metadata};
use crate::scan::{FoundFile, ScanError, find_photos};

/// Number of photos whose metadata is read between two progress reports.
const READ_CHUNK: usize = 200;

/// Where a photo's capture time came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSource {
    /// Recorded by the camera in EXIF.
    Exif,
    /// No usable EXIF date; the file's modification time is used instead.
    FileModified,
}

/// One photo on the timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Photo {
    /// Full path of the file.
    pub path: PathBuf,
    /// File size in bytes.
    pub size: u64,
    /// Last modification time of the file, if the file system reports one.
    pub modified: Option<SystemTime>,
    /// Capture time as local wall-clock time.
    pub taken_at: NaiveDateTime,
    /// Where [`Photo::taken_at`] came from.
    pub time_source: TimeSource,
    /// EXIF orientation.
    pub orientation: Orientation,
    /// Camera description, e.g. `Google Pixel 8`.
    pub camera: Option<String>,
}

/// The photos of one folder tree, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Timeline {
    /// Folder the timeline was built from.
    pub root: PathBuf,
    /// Photos sorted by capture time, newest first.
    pub photos: Vec<Photo>,
    /// Photos in "Odrzucone" folders, sorted the same way. Not shown, but
    /// series are found including them.
    pub rejected: Vec<Photo>,
    /// Entries that could not be read while searching.
    pub unreadable_entries: u64,
}

impl Timeline {
    /// Number of photos whose capture time comes from the file date.
    #[must_use]
    pub fn without_exif_date(&self) -> usize {
        self.photos
            .iter()
            .filter(|p| p.time_source == TimeSource::FileModified)
            .count()
    }

    /// Takes the photo at `path` off the timeline; returns its former position
    /// and the photo, or `None` if it is not on the timeline.
    pub fn remove(&mut self, path: &Path) -> Option<(usize, Photo)> {
        let position = self.photos.iter().position(|photo| photo.path == path)?;
        Some((position, self.photos.remove(position)))
    }

    /// Puts `photo` on the timeline at its place in the timeline order and
    /// returns its position.
    pub fn insert(&mut self, photo: Photo) -> usize {
        let position = self
            .photos
            .partition_point(|other| compare_photos(other, &photo) == Ordering::Less);
        self.photos.insert(position, photo);
        position
    }
}

/// Order of photos on a timeline: newest first, ties broken by path.
#[must_use]
pub fn compare_photos(a: &Photo, b: &Photo) -> Ordering {
    b.taken_at
        .cmp(&a.taken_at)
        .then_with(|| a.path.cmp(&b.path))
}

/// Reads one photo as it would appear on a timeline, e.g. one moved back
/// from the rejected folder.
///
/// # Errors
///
/// Returns an error if the file cannot be inspected.
pub fn read_photo(path: &Path) -> std::io::Result<Photo> {
    let meta = std::fs::metadata(path)?;
    Ok(to_photo(&FoundFile {
        path: path.to_path_buf(),
        size: meta.len(),
        modified: meta.modified().ok(),
    }))
}

/// Progress while building a timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineProgress {
    /// Searching folders; `found` photos so far.
    Searching {
        /// Photos found so far.
        found: u64,
    },
    /// Reading capture times of the found photos.
    Reading {
        /// Photos read so far.
        done: u64,
        /// Photos to read in total.
        total: u64,
    },
}

/// Finds all photos under `root`, reads their capture times in parallel and
/// sorts them newest first (ties broken by path).
///
/// # Errors
///
/// Returns an error if `root` does not exist, is not a folder, or cannot be
/// inspected.
pub fn build_timeline(
    root: &Path,
    mut on_progress: impl FnMut(TimelineProgress),
) -> Result<Timeline, ScanError> {
    let found = find_photos(root, |p| {
        on_progress(TimelineProgress::Searching {
            found: p.photos_found,
        });
    })?;

    let total = found.photos.len() as u64;
    let mut photos = Vec::with_capacity(found.photos.len());
    on_progress(TimelineProgress::Reading { done: 0, total });
    for chunk in found.photos.chunks(READ_CHUNK) {
        photos.par_extend(chunk.par_iter().map(to_photo));
        on_progress(TimelineProgress::Reading {
            done: photos.len() as u64,
            total,
        });
    }

    photos.sort_by(compare_photos);
    // Usually few; read after the shown photos, without progress reports.
    let mut rejected: Vec<Photo> = found.rejected.par_iter().map(to_photo).collect();
    rejected.sort_by(compare_photos);

    Ok(Timeline {
        root: root.to_path_buf(),
        photos,
        rejected,
        unreadable_entries: found.unreadable_entries,
    })
}

fn to_photo(file: &FoundFile) -> Photo {
    let meta = read_photo_metadata(&file.path);
    let (taken_at, time_source) = match meta.captured {
        Some(captured) => (captured.local, TimeSource::Exif),
        None => (local_time(file.modified), TimeSource::FileModified),
    };
    Photo {
        path: file.path.clone(),
        size: file.size,
        modified: file.modified,
        taken_at,
        time_source,
        orientation: meta.orientation,
        camera: meta.camera,
    }
}

fn local_time(time: Option<SystemTime>) -> NaiveDateTime {
    DateTime::<Local>::from(time.unwrap_or(SystemTime::UNIX_EPOCH)).naive_local()
}
