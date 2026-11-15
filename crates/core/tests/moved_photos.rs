// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use sito_core::moved::MovedPhoto;
use sito_core::photo_cache::{PhotoCache, SharedCache};
use sito_core::timeline::{CachedScan, ReadMode, TimelineProgress, build_timeline_cached};
use tempfile::TempDir;

/// Copies the EXIF fixtures into `dir`, keeping their modification times.
fn fill(dir: &Path) {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/exif");
    fs::create_dir_all(dir).unwrap();
    for entry in fs::read_dir(fixtures).unwrap() {
        copy_keeping_time(&entry.unwrap().path(), dir);
    }
}

/// Copies `file` into `dir` with the same modification time.
fn copy_keeping_time(file: &Path, dir: &Path) {
    let target = dir.join(file.file_name().unwrap());
    fs::copy(file, &target).unwrap();
    let modified = fs::metadata(file).unwrap().modified().unwrap();
    File::options()
        .write(true)
        .open(&target)
        .unwrap()
        .set_modified(modified)
        .unwrap();
}

/// Scans `root` with `cache`; returns the scan and how many photos it read.
fn scan(root: &Path, cache: &SharedCache) -> (CachedScan, u64) {
    let mut read = 0;
    let scan = build_timeline_cached(
        root,
        cache,
        ReadMode::Full,
        || false,
        |progress| {
            if let TimelineProgress::Reading { total, .. } = progress {
                read = total;
            }
        },
    )
    .unwrap();
    (scan, read)
}

fn new_cache() -> SharedCache {
    SharedCache::new(Some(PhotoCache::open_in_memory().unwrap()))
}

fn sorted(mut moves: Vec<MovedPhoto>) -> Vec<MovedPhoto> {
    moves.sort_by(|a, b| a.from.cmp(&b.from));
    moves
}

/// The moves of every fixture from `from` to `to`.
fn all_moved(from: &Path, to: &Path) -> Vec<MovedPhoto> {
    let mut names: Vec<_> = fs::read_dir(to)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().ends_with(".jpg"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| MovedPhoto {
            from: from.join(&name),
            to: to.join(&name),
        })
        .collect()
}

#[test]
fn a_renamed_subfolder_is_not_read_again() {
    let root = TempDir::new().unwrap();
    fill(&root.path().join("2023"));
    let cache = new_cache();
    let (first, _) = scan(root.path(), &cache);

    fs::rename(root.path().join("2023"), root.path().join("Rok 2023")).unwrap();
    let (rescan, read) = scan(root.path(), &cache);

    assert_eq!(read, 0);
    assert_eq!(rescan.timeline.photos.len(), first.timeline.photos.len());
    assert!(
        rescan
            .timeline
            .photos
            .iter()
            .all(|photo| photo.path.starts_with(root.path().join("Rok 2023")))
    );
    assert_eq!(
        sorted(rescan.moves),
        all_moved(&root.path().join("2023"), &root.path().join("Rok 2023"))
    );
    // The cache holds the new paths only, so the next scan finds no moves.
    let (again, read) = scan(root.path(), &cache);
    assert_eq!((again.moves, read), (vec![], 0));
}

#[test]
fn a_renamed_scanned_folder_opened_under_its_new_name_is_not_read_again() {
    let parent = TempDir::new().unwrap();
    let old = parent.path().join("Zdjęcia z roku 2023");
    let new = parent.path().join("2023");
    fill(&old);
    let cache = new_cache();
    scan(&old, &cache);

    fs::rename(&old, &new).unwrap();
    let (rescan, read) = scan(&new, &cache);

    assert_eq!(read, 0);
    assert_eq!(sorted(rescan.moves), all_moved(&old, &new));
    let cached = cache.lock().unwrap();
    let cached = cached.as_ref().unwrap();
    assert_eq!(cached.photos_under(&old).unwrap(), vec![]);
}

#[test]
fn a_copy_of_a_scanned_folder_is_not_read_and_not_a_move() {
    let parent = TempDir::new().unwrap();
    let original = parent.path().join("original");
    let copy = parent.path().join("copy");
    fill(&original);
    let cache = new_cache();
    scan(&original, &cache);

    fill(&copy);
    let (scanned, read) = scan(&copy, &cache);

    assert_eq!((scanned.moves, read), (vec![], 0));
    let cached = cache.lock().unwrap();
    let originals = cached.as_ref().unwrap().photos_under(&original).unwrap();
    assert_eq!(originals.len(), 7);
}

#[test]
fn a_photo_moved_into_a_rejected_folder_is_not_listed_as_a_move() {
    let root = TempDir::new().unwrap();
    fill(root.path());
    let cache = new_cache();
    scan(root.path(), &cache);

    fs::create_dir(root.path().join("Odrzucone")).unwrap();
    fs::rename(
        root.path().join("no-exif.jpg"),
        root.path().join("Odrzucone/no-exif.jpg"),
    )
    .unwrap();
    let (rescan, read) = scan(root.path(), &cache);

    assert_eq!((rescan.moves, read), (vec![], 0));
    assert_eq!(rescan.timeline.rejected.len(), 1);
}

#[test]
fn two_photos_with_the_same_identity_are_read_again() {
    let root = TempDir::new().unwrap();
    let photo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/exif/no-exif.jpg");
    for dir in ["a", "b"] {
        fs::create_dir(root.path().join(dir)).unwrap();
        copy_keeping_time(&photo, &root.path().join(dir));
    }
    let cache = new_cache();
    scan(root.path(), &cache);

    fs::rename(root.path().join("a"), root.path().join("c")).unwrap();
    fs::rename(root.path().join("b"), root.path().join("d")).unwrap();
    let (rescan, read) = scan(root.path(), &cache);

    assert_eq!((rescan.moves, read), (vec![], 2));
}
