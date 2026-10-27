// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::NaiveDate;
use image::{Rgb, RgbImage};
use sito_core::metadata::Orientation;
use sito_core::thumbnail::{ThumbnailCache, ThumbnailError};
use sito_core::timeline::{Photo, TimeSource};
use tempfile::TempDir;
use walkdir::WalkDir;

fn photo(path: PathBuf) -> Photo {
    Photo {
        size: fs::metadata(&path).map_or(0, |m| m.len()),
        modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_717_236_000)),
        path,
        taken_at: NaiveDate::from_ymd_opt(2024, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap(),
        time_source: TimeSource::Exif,
        orientation: Orientation::NORMAL,
        camera: None,
    }
}

fn write_photo(dir: &Path) -> PathBuf {
    let path = dir.join("photo.jpg");
    RgbImage::from_pixel(800, 600, Rgb([30, 120, 200]))
        .save(&path)
        .unwrap();
    path
}

fn files_in(dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .collect()
}

#[test]
fn stores_a_new_thumbnail_in_a_subfolder() {
    let photos = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    let cache = ThumbnailCache::new(cache_dir.path().join("thumbnails"));
    let photo = photo(write_photo(photos.path()));

    let bytes = cache.get_or_create(&photo).unwrap();

    let stored = cache.path_for(&photo);
    assert_eq!(fs::read(&stored).unwrap(), bytes);
    assert_eq!(stored.parent().unwrap().parent().unwrap(), cache.dir());
    assert_eq!(files_in(cache.dir()), [stored], "no temporary files left");
}

#[test]
fn serves_a_cached_thumbnail_without_rendering_again() {
    let photos = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    let cache = ThumbnailCache::new(cache_dir.path());
    let photo = photo(write_photo(photos.path()));
    cache.get_or_create(&photo).unwrap();
    fs::write(cache.path_for(&photo), b"cached").unwrap();

    assert_eq!(cache.get_or_create(&photo).unwrap(), b"cached");
}

#[test]
fn key_changes_when_the_photo_changes() {
    let cache = ThumbnailCache::new("cache");
    let original = photo(PathBuf::from("gallery/a.jpg"));
    let key = cache.path_for(&original);

    let mut same = original.clone();
    same.taken_at += chrono::Duration::hours(1);
    assert_eq!(
        cache.path_for(&same),
        key,
        "only path, size and mtime count"
    );

    let mut resized = original.clone();
    resized.size += 1;
    let mut touched = original.clone();
    touched.modified = Some(SystemTime::UNIX_EPOCH);
    let mut moved = original.clone();
    moved.path = PathBuf::from("gallery/b.jpg");
    for changed in [resized, touched, moved] {
        assert_ne!(cache.path_for(&changed), key, "{changed:?}");
    }
}

#[test]
fn does_not_store_anything_for_broken_photos() {
    let photos = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    let cache = ThumbnailCache::new(cache_dir.path());
    let path = photos.path().join("broken.jpg");
    fs::write(&path, b"not a JPEG").unwrap();

    let err = cache.get_or_create(&photo(path)).unwrap_err();

    assert!(matches!(err, ThumbnailError::Decode { .. }), "{err:?}");
    assert_eq!(files_in(cache_dir.path()), Vec::<PathBuf>::new());
}

#[test]
fn is_safe_to_use_from_several_threads_at_once() {
    let photos = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    let cache = ThumbnailCache::new(cache_dir.path());
    let photo = photo(write_photo(photos.path()));

    let results: Vec<_> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|_| s.spawn(|| cache.get_or_create(&photo)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    let first = results[0].as_ref().unwrap();
    for result in &results {
        assert_eq!(result.as_ref().unwrap(), first);
    }
    assert_eq!(files_in(cache_dir.path()), [cache.path_for(&photo)]);
}

/// `count` small photos in `dir`, each with its thumbnail cached.
fn cached_photos(dir: &Path, cache: &ThumbnailCache, count: usize) -> Vec<Photo> {
    (0..count)
        .map(|i| {
            let path = dir.join(format!("photo-{i}.jpg"));
            RgbImage::from_pixel(64, 48, Rgb([30, 120, 200]))
                .save(&path)
                .unwrap();
            let photo = photo(path);
            cache.get_or_create(&photo).unwrap();
            photo
        })
        .collect()
}

#[test]
fn clean_removes_thumbnails_of_photos_no_longer_known() {
    let photos_dir = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    let cache = ThumbnailCache::new(cache_dir.path());
    let photos = cached_photos(photos_dir.path(), &cache, 3);

    let report = cache.clean(&photos[..2], u64::MAX).unwrap();

    assert_eq!(report.orphans_removed, 1);
    assert_eq!(report.oldest_removed, 0);
    assert!(cache.path_for(&photos[0]).exists());
    assert!(cache.path_for(&photos[1]).exists());
    assert!(!cache.path_for(&photos[2]).exists());
}

#[test]
fn clean_keeps_the_cache_under_the_limit_removing_the_oldest_first() {
    let photos_dir = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    let cache = ThumbnailCache::new(cache_dir.path());
    let photos = cached_photos(photos_dir.path(), &cache, 4);
    // Make photo 0 the oldest thumbnail and photo 3 the newest.
    let now = SystemTime::now();
    for (i, photo) in photos.iter().enumerate() {
        let age = Duration::from_secs(3600 * (4 - i as u64));
        fs::File::options()
            .write(true)
            .open(cache.path_for(photo))
            .unwrap()
            .set_modified(now - age)
            .unwrap();
    }
    let size = fs::metadata(cache.path_for(&photos[0])).unwrap().len();

    // Room for two and a half thumbnails: the two oldest go.
    let report = cache.clean(&photos, size * 5 / 2).unwrap();

    assert_eq!(report.oldest_removed, 2);
    assert!(!cache.path_for(&photos[0]).exists());
    assert!(!cache.path_for(&photos[1]).exists());
    assert!(cache.path_for(&photos[2]).exists());
    assert!(cache.path_for(&photos[3]).exists());
    assert!(report.bytes_left <= size * 5 / 2);
}

#[test]
fn clean_never_touches_files_it_did_not_create() {
    let cache_dir = TempDir::new().unwrap();
    let cache = ThumbnailCache::new(cache_dir.path());
    let foreign = [
        cache_dir.path().join("notes.txt"),
        cache_dir.path().join("ab").join("holiday.jpg"),
        cache_dir
            .path()
            .join("photos")
            .join(format!("{}.jpg", "a".repeat(64))),
    ];
    for file in &foreign {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, b"keep me").unwrap();
    }

    let report = cache.clean(&[], 0).unwrap();

    assert_eq!(report.orphans_removed, 0);
    for file in &foreign {
        assert!(file.exists(), "{}", file.display());
    }
}

#[test]
fn clean_of_a_cache_never_written_does_nothing() {
    let cache_dir = TempDir::new().unwrap();
    let cache = ThumbnailCache::new(cache_dir.path().join("thumbnails"));

    assert_eq!(cache.clean(&[], 0).unwrap().bytes_left, 0);
}
