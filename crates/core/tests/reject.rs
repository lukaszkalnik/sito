// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

use sito_core::reject::{RejectError, reject, restore};
use tempfile::TempDir;

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn moves_the_photo_into_odrzucone_next_to_it() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("Camera/IMG_1.jpg");
    write(&photo, "photo");

    let rejected = reject(&photo).unwrap();

    assert_eq!(rejected.path, dir.path().join("Camera/Odrzucone/IMG_1.jpg"));
    assert!(rejected.created_dir);
    assert!(!photo.exists());
    assert_eq!(fs::read_to_string(&rejected.path).unwrap(), "photo");
}

#[test]
fn tells_whether_the_folder_already_existed() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("Odrzucone")).unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "photo");

    assert!(!reject(&photo).unwrap().created_dir);
}

#[test]
fn never_overwrites_an_earlier_rejected_photo() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&dir.path().join("Odrzucone/IMG_1.jpg"), "older");
    write(&dir.path().join("Odrzucone/IMG_1 (2).jpg"), "old");
    write(&photo, "new");

    let rejected = reject(&photo).unwrap();

    assert_eq!(rejected.path, dir.path().join("Odrzucone/IMG_1 (3).jpg"));
    assert_eq!(
        fs::read_to_string(dir.path().join("Odrzucone/IMG_1.jpg")).unwrap(),
        "older"
    );
    assert_eq!(fs::read_to_string(&rejected.path).unwrap(), "new");
}

#[test]
fn restores_the_photo_and_removes_the_empty_folder_when_asked() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "photo");
    let rejected = reject(&photo).unwrap();

    restore(&rejected.path, &photo, true).unwrap();

    assert_eq!(fs::read_to_string(&photo).unwrap(), "photo");
    assert!(!dir.path().join("Odrzucone").exists());
}

#[test]
fn keeps_an_empty_folder_that_is_not_to_be_removed() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "photo");
    let rejected = reject(&photo).unwrap();

    restore(&rejected.path, &photo, false).unwrap();

    assert!(photo.exists());
    assert!(dir.path().join("Odrzucone").is_dir());
}

#[test]
fn keeps_the_folder_while_other_photos_are_in_it() {
    let dir = TempDir::new().unwrap();
    let first = dir.path().join("IMG_1.jpg");
    let second = dir.path().join("IMG_2.jpg");
    write(&first, "1");
    write(&second, "2");
    let rejected_first = reject(&first).unwrap();
    reject(&second).unwrap();

    restore(&rejected_first.path, &first, true).unwrap();

    assert!(first.exists());
    assert!(dir.path().join("Odrzucone/IMG_2.jpg").exists());
}

#[test]
fn restores_a_renamed_photo_under_its_original_name() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&dir.path().join("Odrzucone/IMG_1.jpg"), "other");
    write(&photo, "mine");
    let rejected = reject(&photo).unwrap();

    restore(&rejected.path, &photo, true).unwrap();

    assert_eq!(fs::read_to_string(&photo).unwrap(), "mine");
    assert_eq!(
        fs::read_to_string(dir.path().join("Odrzucone/IMG_1.jpg")).unwrap(),
        "other"
    );
}

#[test]
fn refuses_to_overwrite_a_new_file_with_the_original_name() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "rejected");
    let rejected = reject(&photo).unwrap();
    write(&photo, "new file");

    let err = restore(&rejected.path, &photo, true).unwrap_err();

    assert!(matches!(err, RejectError::Occupied(path) if path == photo));
    assert_eq!(fs::read_to_string(&photo).unwrap(), "new file");
    assert!(rejected.path.exists());
}

#[test]
fn reports_a_rejected_photo_that_is_gone() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    let gone = dir.path().join("Odrzucone/IMG_1.jpg");

    let err = restore(&gone, &photo, true).unwrap_err();

    assert!(matches!(err, RejectError::Missing(path) if path == gone));
}

#[test]
fn reports_a_photo_that_cannot_be_moved() {
    let dir = TempDir::new().unwrap();

    let err = reject(&dir.path().join("missing.jpg")).unwrap_err();

    assert!(matches!(err, RejectError::Io { .. }));
}
