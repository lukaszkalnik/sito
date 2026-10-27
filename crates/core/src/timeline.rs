//! All photos of a folder tree, with capture times, sorted newest first.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local, NaiveDateTime};
use rayon::prelude::*;

use crate::metadata::{Orientation, read_photo_metadata};
use crate::photo_cache::{CacheError, PhotoCache};
use crate::scan::{FoundFile, ScanError, find_photos, is_in_rejected_dir};

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
    on_progress: impl FnMut(TimelineProgress),
) -> Result<Timeline, ScanError> {
    build_timeline_cached(root, None, on_progress)
}

/// Like [`build_timeline`], but takes photos whose size and modification time
/// are unchanged from `cache` instead of reading them, and stores what it
/// read; photos under `root` that are gone are removed from the cache. Only
/// the photos that really need reading count in the reading progress.
///
/// The cache only speeds things up: if it cannot be read or written, every
/// photo is read as without one.
///
/// # Errors
///
/// Returns an error if `root` does not exist, is not a folder, or cannot be
/// inspected.
pub fn build_timeline_cached(
    root: &Path,
    cache: Option<&mut PhotoCache>,
    mut on_progress: impl FnMut(TimelineProgress),
) -> Result<Timeline, ScanError> {
    let found = find_photos(root, |p| {
        on_progress(TimelineProgress::Searching {
            found: p.photos_found,
        });
    })?;

    let known: HashMap<PathBuf, Photo> = cache
        .as_deref()
        .and_then(|cache| cache.photos_under(root).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|photo| (photo.path.clone(), photo))
        .collect();
    let cached = |file: &FoundFile| {
        known
            .get(&file.path)
            .filter(|photo| photo.size == file.size && photo.modified == file.modified)
            .cloned()
    };

    let mut photos = Vec::with_capacity(found.photos.len());
    let mut to_read = Vec::new();
    for file in found.photos.iter().chain(&found.rejected) {
        match cached(file) {
            Some(photo) => photos.push(photo),
            None => to_read.push(file),
        }
    }

    let total = to_read.len() as u64;
    let mut read = Vec::with_capacity(to_read.len());
    on_progress(TimelineProgress::Reading { done: 0, total });
    for chunk in to_read.chunks(READ_CHUNK) {
        read.par_extend(chunk.par_iter().map(|file| to_photo(file)));
        on_progress(TimelineProgress::Reading {
            done: read.len() as u64,
            total,
        });
    }

    if let Some(cache) = cache {
        // Failing to cache only means reading these photos again next time.
        let _ = cache.store(&read);
        let present: HashSet<PathBuf> = found
            .photos
            .iter()
            .chain(&found.rejected)
            .map(|file| file.path.clone())
            .collect();
        let _ = cache.forget_missing(root, &present);
    }

    photos.extend(read);
    let (mut rejected, mut photos): (Vec<Photo>, Vec<Photo>) = photos
        .into_iter()
        .partition(|photo| is_in_rejected_dir(&photo.path, root));
    photos.sort_by(compare_photos);
    rejected.sort_by(compare_photos);

    Ok(Timeline {
        root: root.to_path_buf(),
        photos,
        rejected,
        unreadable_entries: found.unreadable_entries,
    })
}

/// The timeline of `root` as last scanned, from `cache` alone without
/// touching any file, so a folder can be shown at once while it is rescanned.
/// Empty if the folder was never scanned.
///
/// # Errors
///
/// Returns an error if the cache cannot be read.
pub fn cached_timeline(root: &Path, cache: &PhotoCache) -> Result<Timeline, CacheError> {
    let (mut rejected, mut photos): (Vec<Photo>, Vec<Photo>) = cache
        .photos_under(root)?
        .into_iter()
        .partition(|photo| is_in_rejected_dir(&photo.path, root));
    photos.sort_by(compare_photos);
    rejected.sort_by(compare_photos);
    Ok(Timeline {
        root: root.to_path_buf(),
        photos,
        rejected,
        unreadable_entries: 0,
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
