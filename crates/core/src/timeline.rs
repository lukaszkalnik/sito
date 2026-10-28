//! All photos of a folder tree, with capture times, sorted newest first.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local, NaiveDateTime};
use rayon::prelude::*;

use crate::metadata::{Orientation, is_plausible_capture_time, read_photo_metadata};
use crate::photo_cache::{CacheError, PhotoCache};
use crate::scan::{FileStamp, FoundFile, FoundSidecar, ScanError, find_photos, is_in_rejected_dir};
use crate::takeout::{ChosenTime, TakeoutUse, choose_time, find_sidecar, read_sidecar};

/// Number of photos whose metadata is read between two progress reports.
const READ_CHUNK: usize = 200;

/// Where a photo's capture time came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSource {
    /// Recorded by the camera in EXIF.
    Exif,
    /// From the photo's Google Takeout date file, for the given reason.
    Takeout(TakeoutUse),
    /// No usable capture time; the file's modification time is used instead.
    FileModified,
}

impl TimeSource {
    /// Whether the time is when the photo was taken (from EXIF or Takeout),
    /// not just a file date.
    #[must_use]
    pub fn is_capture_time(self) -> bool {
        match self {
            Self::Exif | Self::Takeout(_) => true,
            Self::FileModified => false,
        }
    }
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
    /// Size and modification time of the photo's Takeout date file when it
    /// was read, `None` without one.
    pub sidecar: Option<FileStamp>,
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
    /// Number of photos without a capture time (dated by the file date).
    #[must_use]
    pub fn without_date(&self) -> usize {
        self.photos
            .iter()
            .filter(|p| !p.time_source.is_capture_time())
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
/// from the rejected folder, with the Takeout date file of its folder.
///
/// # Errors
///
/// Returns an error if the file cannot be inspected.
pub fn read_photo(path: &Path) -> std::io::Result<Photo> {
    let meta = std::fs::metadata(path)?;
    let sidecar = find_sidecar(path).and_then(|sidecar| {
        let meta = std::fs::metadata(&sidecar).ok()?;
        Some(FoundSidecar {
            path: sidecar,
            stamp: FileStamp {
                size: meta.len(),
                modified: meta.modified().ok(),
            },
        })
    });
    Ok(to_photo(&FoundFile {
        path: path.to_path_buf(),
        size: meta.len(),
        modified: meta.modified().ok(),
        sidecar,
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

    let all_files: Vec<&FoundFile> = found.photos.iter().chain(&found.rejected).collect();
    let (mut photos, to_read) = split_cached(&all_files, cache.as_deref(), root);
    let read = read_photos(&to_read, &mut on_progress);

    if let Some(cache) = cache {
        update_cache(cache, root, &read, &all_files);
    }

    photos.extend(read);
    Ok(sorted_timeline(root, photos, found.unreadable_entries))
}

/// Splits `files` into the photos still up to date in `cache` and the files
/// that need reading.
fn split_cached<'a>(
    files: &[&'a FoundFile],
    cache: Option<&PhotoCache>,
    root: &Path,
) -> (Vec<Photo>, Vec<&'a FoundFile>) {
    let known: HashMap<PathBuf, Photo> = cache
        .and_then(|cache| cache.photos_under(root).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|photo| (photo.path.clone(), photo))
        .collect();
    let mut photos = Vec::with_capacity(files.len());
    let mut to_read = Vec::new();
    for &file in files {
        match known
            .get(&file.path)
            .filter(|photo| is_unchanged(photo, file))
        {
            Some(photo) => photos.push(photo.clone()),
            None => to_read.push(file),
        }
    }
    (photos, to_read)
}

/// Whether `file` and its Takeout date file are as cached in `photo`.
fn is_unchanged(photo: &Photo, file: &FoundFile) -> bool {
    photo.size == file.size
        && photo.modified == file.modified
        && photo.sidecar == file.sidecar.as_ref().map(|sidecar| sidecar.stamp)
}

/// Reads the metadata of `files` in parallel, reporting progress after each
/// chunk of [`READ_CHUNK`] files.
fn read_photos(files: &[&FoundFile], on_progress: &mut impl FnMut(TimelineProgress)) -> Vec<Photo> {
    let total = files.len() as u64;
    let mut read = Vec::with_capacity(files.len());
    on_progress(TimelineProgress::Reading { done: 0, total });
    for chunk in files.chunks(READ_CHUNK) {
        read.par_extend(chunk.par_iter().map(|file| to_photo(file)));
        on_progress(TimelineProgress::Reading {
            done: read.len() as u64,
            total,
        });
    }
    read
}

/// Stores the freshly `read` photos in `cache` and forgets cached photos
/// under `root` that are no longer among `present`.
fn update_cache(cache: &mut PhotoCache, root: &Path, read: &[Photo], present: &[&FoundFile]) {
    // Failing to cache only means reading these photos again next time.
    let _ = cache.store(read);
    let present: HashSet<PathBuf> = present.iter().map(|file| file.path.clone()).collect();
    let _ = cache.forget_missing(root, &present);
}

/// Builds the timeline of `root` from unsorted `photos`, separating the
/// rejected ones and sorting both lists with [`compare_photos`].
fn sorted_timeline(root: &Path, photos: Vec<Photo>, unreadable_entries: u64) -> Timeline {
    let (mut rejected, mut photos): (Vec<Photo>, Vec<Photo>) = photos
        .into_iter()
        .partition(|photo| is_in_rejected_dir(&photo.path, root));
    photos.sort_by(compare_photos);
    rejected.sort_by(compare_photos);
    Timeline {
        root: root.to_path_buf(),
        photos,
        rejected,
        unreadable_entries,
    }
}

/// The timeline of `root` as last scanned, from `cache` alone without
/// touching any file, so a folder can be shown at once while it is rescanned.
/// Empty if the folder was never scanned.
///
/// # Errors
///
/// Returns an error if the cache cannot be read.
pub fn cached_timeline(root: &Path, cache: &PhotoCache) -> Result<Timeline, CacheError> {
    Ok(sorted_timeline(root, cache.photos_under(root)?, 0))
}

fn to_photo(file: &FoundFile) -> Photo {
    let meta = read_photo_metadata(&file.path);
    let now = Local::now().naive_local();
    // A date file that cannot be read leaves the photo with its own date.
    let takeout = file
        .sidecar
        .as_ref()
        .and_then(|sidecar| read_sidecar(&sidecar.path).ok());
    let (taken_at, time_source) = match (takeout, meta.captured) {
        (Some(takeout), exif) => match choose_time(exif, takeout.taken, now, |utc| {
            utc.with_timezone(&Local).naive_local()
        }) {
            ChosenTime::Exif(time) => (time, TimeSource::Exif),
            ChosenTime::Takeout(time, reason) => (time, TimeSource::Takeout(reason)),
        },
        (None, Some(exif)) if is_plausible_capture_time(exif.local, now) => {
            (exif.local, TimeSource::Exif)
        }
        (None, _) => (local_time(file.modified), TimeSource::FileModified),
    };
    Photo {
        path: file.path.clone(),
        size: file.size,
        modified: file.modified,
        taken_at,
        time_source,
        orientation: meta.orientation,
        camera: meta.camera,
        sidecar: file.sidecar.as_ref().map(|sidecar| sidecar.stamp),
    }
}

fn local_time(time: Option<SystemTime>) -> NaiveDateTime {
    DateTime::<Local>::from(time.unwrap_or(SystemTime::UNIX_EPOCH)).naive_local()
}
