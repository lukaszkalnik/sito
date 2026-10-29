// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use sito_core::photo_cache::PhotoCache;
use sito_core::reject::{reject, restore};
use sito_core::takeout::TakeoutUse;
use sito_core::timeline::{
    Completion, ExifInfo, Photo, ReadMode, TimeSource, build_timeline, build_timeline_cached,
    cached_timeline, complete_pending, read_photo,
};
use tempfile::TempDir;

/// A copy of the Takeout album fixture (see fixtures/README.md).
fn album() -> TempDir {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/takeout/Album");
    let dir = TempDir::new().unwrap();
    for entry in fs::read_dir(fixtures).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dir.path().join(entry.file_name())).unwrap();
    }
    dir
}

fn at(text: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").unwrap()
}

/// A UTC time as wall-clock time on this computer, like a Takeout date is shown.
fn local(utc: &str) -> NaiveDateTime {
    Utc.from_utc_datetime(&at(utc))
        .with_timezone(&Local)
        .naive_local()
}

fn by_name(photos: &[Photo]) -> HashMap<String, (NaiveDateTime, TimeSource)> {
    photos
        .iter()
        .map(|p| {
            let name = p.path.file_name().unwrap().to_string_lossy().into_owned();
            (name, (p.taken_at, p.time_source))
        })
        .collect()
}

#[test]
fn dates_photos_by_their_takeout_date_files() {
    let dir = album();

    let timeline = build_timeline(dir.path(), |_| {}).unwrap();
    let photos = by_name(&timeline.photos);

    let takeout = TimeSource::Takeout(TakeoutUse::MissingExif);
    let expected = [
        ("no-exif.jpg", local("2024-06-01 10:00:00"), takeout),
        ("IMG_1.jpg", at("2024-06-01 12:00:05"), TimeSource::Exif),
        ("IMG_1(1).jpg", local("2024-06-01 10:00:08"), takeout),
        (
            "scan_1976.jpg",
            local("1976-07-01 10:00:00"),
            TimeSource::Takeout(TakeoutUse::Corrected),
        ),
        (
            "reset_clock.jpg",
            local("2024-06-02 08:00:00"),
            TimeSource::Takeout(TakeoutUse::ImplausibleExif),
        ),
        ("old_export.jpg", local("2024-06-03 08:00:00"), takeout),
        (
            "Screenshot_20240604-120000_Some_Long_App.jpg",
            local("2024-06-04 10:00:00"),
            takeout,
        ),
        ("renamed.jpg", local("2024-06-05 10:00:00"), takeout),
    ];
    for (name, time, source) in expected {
        assert_eq!(photos[name], (time, source), "{name}");
    }
    assert_eq!(
        photos["no-sidecar_reset-clock.jpg"].1,
        TimeSource::FileModified
    );
    assert_eq!(timeline.without_date(), 1);
}

#[test]
fn a_rejected_and_restored_photo_keeps_its_takeout_date() {
    let dir = album();
    let photo = dir.path().join("scan_1976.jpg");

    let rejected = reject(&photo).unwrap();
    let in_odrzucone = read_photo(&rejected.path).unwrap();
    restore(&rejected.path, &photo, &rejected.extra, true).unwrap();
    let back = read_photo(&photo).unwrap();

    let year_1976 = NaiveDate::from_ymd_opt(1976, 7, 1).unwrap();
    for read in [in_odrzucone, back] {
        assert_eq!(read.taken_at.date(), year_1976);
        assert_eq!(read.time_source, TimeSource::Takeout(TakeoutUse::Corrected));
    }
}

/// Scans `dir` in [`ReadMode::TakeoutFirst`] into a fresh cache.
fn quick_scan(dir: &TempDir) -> PhotoCache {
    let mut cache = PhotoCache::open_in_memory().unwrap();
    build_timeline_cached(dir.path(), Some(&mut cache), ReadMode::TakeoutFirst, |_| {}).unwrap();
    cache
}

/// Reads the pending photos of `dir` into `cache`, stopping when `should_stop` says so.
fn complete(dir: &TempDir, cache: &mut PhotoCache, should_stop: impl Fn() -> bool) -> Completion {
    let pending = cache.pending_under(dir.path()).unwrap();
    complete_pending(
        &pending,
        |read| cache.store(read).unwrap(),
        |_, _| {},
        should_stop,
    )
}

#[test]
fn a_quick_first_scan_dates_photos_from_their_date_files_alone() {
    let dir = album();

    let cache = quick_scan(&dir);
    let timeline = cached_timeline(dir.path(), &cache).unwrap();

    let by_path: HashMap<String, &Photo> = timeline
        .photos
        .iter()
        .map(|p| {
            (
                p.path.file_name().unwrap().to_string_lossy().into_owned(),
                p,
            )
        })
        .collect();
    let img = by_path["IMG_1.jpg"];
    assert_eq!(img.exif, ExifInfo::Pending);
    assert_eq!(
        (img.taken_at, img.time_source),
        (
            local("2024-06-01 10:00:05"),
            TimeSource::Takeout(TakeoutUse::MissingExif)
        )
    );
    assert!(matches!(
        by_path["no-sidecar_reset-clock.jpg"].exif,
        ExifInfo::Read { .. }
    ));
}

#[test]
fn completing_a_quick_scan_gives_the_full_scan_result() {
    let dir = album();
    let mut cache = quick_scan(&dir);

    let completion = complete(&dir, &mut cache, || false);

    assert_eq!(completion, Completion::Finished);
    assert_eq!(cache.pending_under(dir.path()).unwrap(), Vec::new());
    let full = build_timeline(dir.path(), |_| {}).unwrap();
    assert_eq!(
        cached_timeline(dir.path(), &cache).unwrap().photos,
        full.photos
    );
}

#[test]
fn a_stopped_completion_resumes_later() {
    let dir = album();
    let mut cache = quick_scan(&dir);
    let pending = cache.pending_under(dir.path()).unwrap().len();

    let stopped = complete(&dir, &mut cache, || true);
    // A rescan keeps the unread photos pending.
    build_timeline_cached(dir.path(), Some(&mut cache), ReadMode::TakeoutFirst, |_| {}).unwrap();

    assert_eq!(stopped, Completion::Stopped);
    assert_eq!(cache.pending_under(dir.path()).unwrap().len(), pending);
    assert_eq!(complete(&dir, &mut cache, || false), Completion::Finished);
    assert_eq!(cache.pending_under(dir.path()).unwrap(), Vec::new());
}
