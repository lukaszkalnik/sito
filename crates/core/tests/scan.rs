// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

use sito_core::scan::{ScanError, ScanProgress, scan};
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
    ] {
        touch(root, file);
    }
    dir
}

#[test]
fn counts_jpegs_and_skips_hidden_and_nomedia_folders() {
    let dir = gallery();

    let summary = scan(dir.path(), |_| {}).unwrap();

    assert_eq!(summary.photos_found, 4);
    assert_eq!(summary.unreadable_entries, 0);
}

#[test]
fn reports_final_progress_matching_summary() {
    let dir = gallery();
    let mut reports: Vec<ScanProgress> = Vec::new();

    let summary = scan(dir.path(), |p| reports.push(p)).unwrap();

    let last = reports.last().expect("at least one progress report");
    assert_eq!(last.photos_found, summary.photos_found);
    assert_eq!(last.entries_seen, summary.entries_seen);
}

#[test]
fn reports_progress_periodically_on_large_trees() {
    let dir = TempDir::new().unwrap();
    for i in 0..1200 {
        touch(dir.path(), &format!("IMG_{i:04}.jpg"));
    }
    let mut reports: Vec<ScanProgress> = Vec::new();

    let summary = scan(dir.path(), |p| reports.push(p)).unwrap();

    assert_eq!(summary.photos_found, 1200);
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

    let summary = scan(dir.path(), |_| {}).unwrap();

    assert_eq!(summary.photos_found, 0);
}

#[test]
fn hidden_root_is_still_scanned() {
    let dir = TempDir::new().unwrap();
    touch(dir.path(), ".hidden-root/IMG_0001.jpg");

    let summary = scan(&dir.path().join(".hidden-root"), |_| {}).unwrap();

    assert_eq!(summary.photos_found, 1);
}

#[test]
fn missing_root_is_not_found() {
    let dir = TempDir::new().unwrap();

    let err = scan(&dir.path().join("missing"), |_| {}).unwrap_err();

    assert!(matches!(err, ScanError::NotFound(_)), "got {err:?}");
}

#[test]
fn file_root_is_not_a_directory() {
    let dir = TempDir::new().unwrap();
    touch(dir.path(), "IMG_0001.jpg");

    let err = scan(&dir.path().join("IMG_0001.jpg"), |_| {}).unwrap_err();

    assert!(matches!(err, ScanError::NotADirectory(_)), "got {err:?}");
}
