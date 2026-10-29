//! Reading one found photo file into a [`Photo`].

use std::time::SystemTime;

use chrono::{DateTime, Local, NaiveDateTime};
use rayon::prelude::*;

use super::{ExifInfo, Photo, ReadMode, TimeSource, TimelineProgress};
use crate::metadata::{is_plausible_capture_time, read_photo_metadata};
use crate::scan::FoundFile;
use crate::takeout::{ChosenTime, choose_time, read_sidecar};

/// Number of photos read between two progress reports.
pub(super) const READ_CHUNK: usize = 200;

/// Reads `files` in parallel as `mode` says, reporting progress after each
/// chunk of [`READ_CHUNK`] files.
pub(super) fn read_photos(
    files: &[&FoundFile],
    mode: ReadMode,
    on_progress: &mut impl FnMut(TimelineProgress),
) -> Vec<Photo> {
    let total = files.len() as u64;
    let mut read = Vec::with_capacity(files.len());
    on_progress(TimelineProgress::Reading { done: 0, total });
    for chunk in files.chunks(READ_CHUNK) {
        read.par_extend(chunk.par_iter().map(|file| match mode {
            ReadMode::Full => to_photo(file),
            ReadMode::TakeoutFirst => to_photo_takeout_first(file),
        }));
        on_progress(TimelineProgress::Reading {
            done: read.len() as u64,
            total,
        });
    }
    read
}

/// The photo with everything read: its EXIF and its Takeout date file.
pub(super) fn to_photo(file: &FoundFile) -> Photo {
    let meta = read_photo_metadata(&file.path);
    let now = Local::now().naive_local();
    // A date file that cannot be read leaves the photo with its own date.
    let takeout = file
        .sidecar
        .as_ref()
        .and_then(|sidecar| read_sidecar(&sidecar.path).ok());
    let (taken_at, time_source) = match (takeout, meta.captured) {
        (Some(takeout), exif) => match choose_time(exif, takeout.taken, now, to_local) {
            ChosenTime::Exif(time) => (time, TimeSource::Exif),
            ChosenTime::Takeout(time, reason) => (time, TimeSource::Takeout(reason)),
        },
        (None, Some(exif)) if is_plausible_capture_time(exif.local, now) => {
            (exif.local, TimeSource::Exif)
        }
        (None, _) => (local_time(file.modified), TimeSource::FileModified),
    };
    Photo {
        exif: ExifInfo::Read {
            orientation: meta.orientation,
            camera: meta.camera,
        },
        ..photo_of(file, taken_at, time_source)
    }
}

/// A photo with a Takeout date file dated from that file alone, without
/// opening the photo (slow on hard disks): its EXIF stays
/// [`ExifInfo::Pending`]. Other photos, and photos whose date file cannot
/// be read, are read fully.
fn to_photo_takeout_first(file: &FoundFile) -> Photo {
    let takeout = file
        .sidecar
        .as_ref()
        .and_then(|sidecar| read_sidecar(&sidecar.path).ok());
    let Some(takeout) = takeout else {
        return to_photo(file);
    };
    let now = Local::now().naive_local();
    let (taken_at, time_source) = match choose_time(None, takeout.taken, now, to_local) {
        ChosenTime::Exif(time) => (time, TimeSource::Exif),
        ChosenTime::Takeout(time, reason) => (time, TimeSource::Takeout(reason)),
    };
    photo_of(file, taken_at, time_source)
}

/// `file` as a photo taken at `taken_at`, its EXIF not read yet.
fn photo_of(file: &FoundFile, taken_at: NaiveDateTime, time_source: TimeSource) -> Photo {
    Photo {
        path: file.path.clone(),
        size: file.size,
        modified: file.modified,
        taken_at,
        time_source,
        exif: ExifInfo::Pending,
        sidecar: file.sidecar.clone(),
        companion: file.companion.clone(),
    }
}

fn to_local(utc: DateTime<chrono::Utc>) -> NaiveDateTime {
    utc.with_timezone(&Local).naive_local()
}

fn local_time(time: Option<SystemTime>) -> NaiveDateTime {
    DateTime::<Local>::from(time.unwrap_or(SystemTime::UNIX_EPOCH)).naive_local()
}
