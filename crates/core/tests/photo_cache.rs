// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::NaiveDate;
use sito_core::metadata::Orientation;
use sito_core::photo_cache::PhotoCache;
use sito_core::timeline::{
    Photo, TimeSource, TimelineProgress, build_timeline, build_timeline_cached, cached_timeline,
};
use tempfile::TempDir;

/// A temp folder with copies of all EXIF fixtures, one of them rejected.
fn gallery() -> TempDir {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/exif");
    let dir = TempDir::new().unwrap();
    for entry in fs::read_dir(fixtures).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dir.path().join(entry.file_name())).unwrap();
    }
    fs::create_dir(dir.path().join("Odrzucone")).unwrap();
    fs::rename(
        dir.path().join("no-exif.jpg"),
        dir.path().join("Odrzucone/no-exif.jpg"),
    )
    .unwrap();
    dir
}

/// How many photos a scan had to read (the total of its reading progress).
fn scan_reading(root: &Path, cache: &mut PhotoCache) -> u64 {
    let mut total = 0;
    build_timeline_cached(root, Some(cache), |progress| {
        if let TimelineProgress::Reading { total: t, .. } = progress {
            total = t;
        }
    })
    .unwrap();
    total
}

#[test]
fn a_rescan_reads_only_new_or_changed_photos() {
    let dir = gallery();
    let mut cache = PhotoCache::open_in_memory().unwrap();
    let all = build_timeline(dir.path(), |_| {}).unwrap();
    let count = (all.photos.len() + all.rejected.len()) as u64;

    assert_eq!(scan_reading(dir.path(), &mut cache), count);
    assert_eq!(scan_reading(dir.path(), &mut cache), 0);

    // A changed modification time means the photo may have changed: read it again.
    let changed = dir.path().join("rotated_orientation-6.jpg");
    File::options()
        .write(true)
        .open(&changed)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(3600))
        .unwrap();
    assert_eq!(scan_reading(dir.path(), &mut cache), 1);
    assert_eq!(scan_reading(dir.path(), &mut cache), 0);
}

#[test]
fn a_cached_scan_gives_the_same_timeline() {
    let dir = gallery();
    let mut cache = PhotoCache::open_in_memory().unwrap();
    let fresh = build_timeline(dir.path(), |_| {}).unwrap();

    build_timeline_cached(dir.path(), Some(&mut cache), |_| {}).unwrap();
    let from_cache = build_timeline_cached(dir.path(), Some(&mut cache), |_| {}).unwrap();

    assert_eq!(from_cache, fresh);
    assert_eq!(from_cache.rejected.len(), 1);
}

#[test]
fn the_cache_alone_shows_the_folder_as_last_scanned() {
    let dir = gallery();
    let mut cache = PhotoCache::open_in_memory().unwrap();
    let scanned = build_timeline_cached(dir.path(), Some(&mut cache), |_| {}).unwrap();

    let instant = cached_timeline(dir.path(), &cache).unwrap();

    assert_eq!(instant.photos, scanned.photos);
    assert_eq!(instant.rejected, scanned.rejected);
    // Another folder was never scanned.
    let other = TempDir::new().unwrap();
    assert_eq!(
        cached_timeline(other.path(), &cache).unwrap().photos,
        Vec::new()
    );
}

#[test]
fn photos_that_are_gone_are_forgotten() {
    let dir = gallery();
    let mut cache = PhotoCache::open_in_memory().unwrap();
    build_timeline_cached(dir.path(), Some(&mut cache), |_| {}).unwrap();
    let before = cache.photos_under(dir.path()).unwrap().len();

    fs::remove_file(dir.path().join("broken-date.jpg")).unwrap();
    build_timeline_cached(dir.path(), Some(&mut cache), |_| {}).unwrap();

    let after = cache.photos_under(dir.path()).unwrap();
    assert_eq!(after.len(), before - 1);
    assert!(after.iter().all(|p| !p.path.ends_with("broken-date.jpg")));
}

#[test]
fn keeps_the_photos_of_other_folders() {
    let first = gallery();
    let second = gallery();
    let mut cache = PhotoCache::open_in_memory().unwrap();
    build_timeline_cached(first.path(), Some(&mut cache), |_| {}).unwrap();
    build_timeline_cached(second.path(), Some(&mut cache), |_| {}).unwrap();

    fs::remove_file(second.path().join("broken-date.jpg")).unwrap();
    build_timeline_cached(second.path(), Some(&mut cache), |_| {}).unwrap();

    assert_eq!(
        cache.photos_under(first.path()).unwrap().len(),
        cache.photos_under(second.path()).unwrap().len() + 1
    );
}

#[test]
fn stores_every_field() {
    let mut cache = PhotoCache::open_in_memory().unwrap();
    let photos = [
        Photo {
            path: PathBuf::from("a.jpg"),
            size: 123,
            modified: Some(
                SystemTime::UNIX_EPOCH + Duration::from_nanos(1_700_000_000_123_456_789),
            ),
            taken_at: NaiveDate::from_ymd_opt(2024, 6, 1)
                .unwrap()
                .and_hms_milli_opt(12, 0, 5, 250)
                .unwrap(),
            time_source: TimeSource::Exif,
            orientation: Orientation::from_exif(6),
            camera: Some("Google Pixel 8".into()),
        },
        Photo {
            path: PathBuf::from("b.jpg"),
            size: 0,
            modified: None,
            taken_at: NaiveDate::from_ymd_opt(2020, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            time_source: TimeSource::FileModified,
            orientation: Orientation::NORMAL,
            camera: None,
        },
    ];

    cache.store(&photos).unwrap();
    let mut back = cache.all().unwrap();
    back.sort_by(|a, b| a.path.cmp(&b.path));

    assert_eq!(back, photos);
}

#[test]
fn keeps_its_contents_across_reopening() {
    let dir = gallery();
    let db = TempDir::new().unwrap();
    let file = db.path().join("photos.db");
    {
        let mut cache = PhotoCache::open(&file).unwrap();
        build_timeline_cached(dir.path(), Some(&mut cache), |_| {}).unwrap();
    }

    let mut cache = PhotoCache::open(&file).unwrap();

    assert_eq!(scan_reading(dir.path(), &mut cache), 0);
}

#[test]
fn a_cache_of_another_version_starts_empty() {
    let dir = gallery();
    let db = TempDir::new().unwrap();
    let file = db.path().join("photos.db");
    {
        let mut cache = PhotoCache::open(&file).unwrap();
        build_timeline_cached(dir.path(), Some(&mut cache), |_| {}).unwrap();
    }
    let conn = rusqlite::Connection::open(&file).unwrap();
    conn.execute("UPDATE meta SET value = 0 WHERE key = 'version'", [])
        .unwrap();
    drop(conn);

    let cache = PhotoCache::open(&file).unwrap();

    assert_eq!(cache.all().unwrap(), Vec::new());
}

#[test]
fn a_damaged_cache_file_is_reported() {
    let db = TempDir::new().unwrap();
    let file = db.path().join("photos.db");
    fs::write(&file, b"this is not a database, just some bytes").unwrap();

    assert!(PhotoCache::open(&file).is_err());
}
