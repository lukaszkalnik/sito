//! All photos of a folder tree, with capture times, sorted newest first.

mod pending;
mod read;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::NaiveDateTime;

pub use pending::{Completion, complete_pending};

use crate::metadata::{Orientation, read_photo_metadata};
use crate::photo_cache::{CacheError, PhotoCache};
use crate::scan::{
    FoundFile, ScanError, StampedFile, find_companion, find_photos, is_in_rejected_dir,
};
use crate::takeout::{TakeoutUse, find_sidecar};
use read::{read_photos, to_photo};

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

/// What the photo's own EXIF says, once it has been read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExifInfo {
    /// Not read yet: a quick first scan dated the photo from its Takeout
    /// date file alone ([`ReadMode::TakeoutFirst`]).
    Pending,
    /// Read from the photo file.
    Read {
        /// EXIF orientation.
        orientation: Orientation,
        /// Camera description, e.g. `Google Pixel 8`.
        camera: Option<String>,
    },
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
    /// The photo's EXIF orientation and camera, or that they are not read yet.
    pub exif: ExifInfo,
    /// The photo's Takeout date file as it was when read, `None` without one.
    pub sidecar: Option<StampedFile>,
    /// The RAW file of a RAW+JPEG pair (this photo is the pair's JPEG).
    pub companion: Option<StampedFile>,
}

impl Photo {
    /// The photo's orientation: from its EXIF, read from the file now if it
    /// is still pending.
    #[must_use]
    pub fn orientation(&self) -> Orientation {
        match &self.exif {
            ExifInfo::Read { orientation, .. } => *orientation,
            ExifInfo::Pending => read_photo_metadata(&self.path).orientation,
        }
    }
}

/// How much of each photo a scan reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadMode {
    /// Every photo file is opened and its EXIF read.
    Full,
    /// Photos with a Takeout date file are dated from it alone and their
    /// EXIF left [`ExifInfo::Pending`] for [`complete_pending`]: opening each
    /// photo is what makes a first scan slow on a hard disk.
    TakeoutFirst,
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
    let companion = find_companion(path);
    let sidecar = find_sidecar(path)
        .or_else(|| find_sidecar(&companion.as_ref()?.path))
        .and_then(|sidecar| StampedFile::read(&sidecar));
    Ok(to_photo(&FoundFile {
        path: path.to_path_buf(),
        size: meta.len(),
        modified: meta.modified().ok(),
        sidecar,
        companion,
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
    build_timeline_cached(root, None, ReadMode::Full, on_progress)
}

/// Like [`build_timeline`], but takes photos whose files are unchanged from
/// `cache` instead of reading them, reads the others as `mode` says, and
/// stores what it read; photos under `root` that are gone are removed from
/// the cache. Only the photos that really need reading count in the reading
/// progress. Cached photos whose EXIF is still pending stay pending.
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
    mode: ReadMode,
    mut on_progress: impl FnMut(TimelineProgress),
) -> Result<Timeline, ScanError> {
    let found = find_photos(root, |p| {
        on_progress(TimelineProgress::Searching {
            found: p.photos_found,
        });
    })?;

    let all_files: Vec<&FoundFile> = found.photos.iter().chain(&found.rejected).collect();
    let (mut photos, to_read) = split_cached(&all_files, cache.as_deref(), root);
    let read = read_photos(&to_read, mode, &mut on_progress);

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

/// Whether `file`, its Takeout date file and its RAW companion are as
/// cached in `photo`.
fn is_unchanged(photo: &Photo, file: &FoundFile) -> bool {
    photo.size == file.size
        && photo.modified == file.modified
        && photo.sidecar == file.sidecar
        && photo.companion == file.companion
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
