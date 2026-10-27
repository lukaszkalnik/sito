// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

use sito_core::scan::{ScanError, SearchProgress, find_photos};
use tempfile::TempDir;

fn touch(root: &Path, relative: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"").unwrap();
}

fn gallery() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    for file in [
        "IMG_0001.jpg",
        "IMG_0002.JPG",
        "Camera/2024/IMG_0003.jpeg",
        "Camera/2024/IMG_0004.JPEG",
        "Camera/2024/notes.txt",
        "Screenshots/screen.png",
        "Camera/clip.mp4",
        // Skipped: hidden folders and folders marked with .nomedia.
        ".thumbnails/thumb.jpg",
        "Camera/.trashed-123/IMG_0005.jpg",
        "WhatsApp/Private/.nomedia",
        "WhatsApp/Private/IMG_0006.jpg",
        "WhatsApp/Private/nested/IMG_0007.jpg",
        // Skipped: rejected photos, in any letter case and at any depth.
        "Odrzucone/IMG_0008.jpg",
        "Camera/2024/odrzucone/IMG_0009.jpg",
        "Camera/ODRZUCONE/deeper/IMG_0010.jpg",
    ] {
        touch(root, file);
    }
    dir
}

#[test]
fn finds_jpegs_and_skips_hidden_nomedia_and_rejected_folders() {
    let dir = gallery();

    let result = find_photos(dir.path(), |_| {}).unwrap();

    let mut names: Vec<String> = result
        .photos
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "IMG_0001.jpg",
            "IMG_0002.JPG",
            "IMG_0003.jpeg",
            "IMG_0004.JPEG"
        ]
    );
    assert_eq!(result.unreadable_entries, 0);
}

#[test]
fn records_size_and_modification_time() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("IMG.jpg"), b"12345").unwrap();

    let result = find_photos(dir.path(), |_| {}).unwrap();

    let file = &result.photos[0];
    assert_eq!(file.size, 5);
    assert!(file.modified.is_some());
}

#[test]
fn reports_final_progress_matching_result() {
    let dir = gallery();
    let mut reports: Vec<SearchProgress> = Vec::new();

    let result = find_photos(dir.path(), |p| reports.push(p)).unwrap();

    let last = reports.last().expect("at least one progress report");
    assert_eq!(last.photos_found, result.photos.len() as u64);
    assert_eq!(last.entries_seen, result.entries_seen);
}

#[test]
fn reports_progress_periodically_on_large_trees() {
    let dir = TempDir::new().unwrap();
    for i in 0..1200 {
        touch(dir.path(), &format!("IMG_{i:04}.jpg"));
    }
    let mut reports: Vec<SearchProgress> = Vec::new();

    let result = find_photos(dir.path(), |p| reports.push(p)).unwrap();

    assert_eq!(result.photos.len(), 1200);
    assert!(reports.len() >= 3, "got {} reports", reports.len());
    assert!(
        reports
            .windows(2)
            .all(|w| w[0].entries_seen <= w[1].entries_seen)
    );
}

#[test]
fn empty_folder_has_no_photos() {
    let dir = TempDir::new().unwrap();

    let result = find_photos(dir.path(), |_| {}).unwrap();

    assert_eq!(result.photos.len(), 0);
}

#[test]
fn hidden_root_is_still_searched() {
    let dir = TempDir::new().unwrap();
    touch(dir.path(), ".hidden-root/IMG_0001.jpg");

    let result = find_photos(&dir.path().join(".hidden-root"), |_| {}).unwrap();

    assert_eq!(result.photos.len(), 1);
}

#[test]
fn missing_root_is_not_found() {
    let dir = TempDir::new().unwrap();

    let err = find_photos(&dir.path().join("missing"), |_| {}).unwrap_err();

    assert!(matches!(err, ScanError::NotFound(_)), "got {err:?}");
}

#[test]
fn file_root_is_not_a_directory() {
    let dir = TempDir::new().unwrap();
    touch(dir.path(), "IMG_0001.jpg");

    let err = find_photos(&dir.path().join("IMG_0001.jpg"), |_| {}).unwrap_err();

    assert!(matches!(err, ScanError::NotADirectory(_)), "got {err:?}");
}

#[test]
fn rejected_root_is_still_searched() {
    let dir = TempDir::new().unwrap();
    touch(dir.path(), "Odrzucone/IMG_0001.jpg");

    let result = find_photos(&dir.path().join("Odrzucone"), |_| {}).unwrap();

    assert_eq!(result.photos.len(), 1);
}
