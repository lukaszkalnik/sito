// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};

use sito_core::reject::{RejectError, abandon, complete, prepare, reject, restore};
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

    restore(&rejected.path, &photo, None, true).unwrap();

    assert_eq!(fs::read_to_string(&photo).unwrap(), "photo");
    assert!(!dir.path().join("Odrzucone").exists());
}

#[test]
fn keeps_an_empty_folder_that_is_not_to_be_removed() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "photo");
    let rejected = reject(&photo).unwrap();

    restore(&rejected.path, &photo, None, false).unwrap();

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

    restore(&rejected_first.path, &first, None, true).unwrap();

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

    restore(&rejected.path, &photo, None, true).unwrap();

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

    let err = restore(&rejected.path, &photo, None, true).unwrap_err();

    assert!(matches!(err, RejectError::Occupied(path) if path == photo));
    assert_eq!(fs::read_to_string(&photo).unwrap(), "new file");
    assert!(rejected.path.exists());
}

#[test]
fn reports_a_rejected_photo_that_is_gone() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    let gone = dir.path().join("Odrzucone/IMG_1.jpg");

    let err = restore(&gone, &photo, None, true).unwrap_err();

    assert!(matches!(err, RejectError::Missing(path) if path == gone));
}

#[test]
fn reports_a_photo_that_cannot_be_moved() {
    let dir = TempDir::new().unwrap();

    let err = reject(&dir.path().join("missing.jpg")).unwrap_err();

    assert!(matches!(err, RejectError::Io { .. }));
}

#[test]
fn prepare_picks_the_target_without_moving() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "photo");

    let prepared = prepare(&photo).unwrap();

    assert_eq!(prepared.path, dir.path().join("Odrzucone/IMG_1.jpg"));
    assert!(photo.exists());
    assert!(!prepared.path.exists());
    complete(&photo, &prepared).unwrap();
    assert!(!photo.exists());
    assert_eq!(fs::read_to_string(&prepared.path).unwrap(), "photo");
}

#[test]
fn complete_refuses_a_target_taken_since_prepare() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "new");
    let prepared = prepare(&photo).unwrap();
    write(&prepared.path, "other");

    assert!(matches!(
        complete(&photo, &prepared),
        Err(RejectError::Occupied(_))
    ));
    assert_eq!(fs::read_to_string(&photo).unwrap(), "new");
    assert_eq!(fs::read_to_string(&prepared.path).unwrap(), "other");
}

#[test]
fn abandon_removes_only_a_folder_prepare_created() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "photo");

    abandon(&prepare(&photo).unwrap());
    assert!(!dir.path().join("Odrzucone").exists());

    fs::create_dir(dir.path().join("Odrzucone")).unwrap();
    abandon(&prepare(&photo).unwrap());
    assert!(dir.path().join("Odrzucone").is_dir());
    assert!(photo.exists());
}

// Only Linux file systems accept names that are not valid UTF-8; macOS
// refuses to create them ("Illegal byte sequence").
#[cfg(target_os = "linux")]
#[test]
fn keeps_non_unicode_names_when_numbering() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = TempDir::new().unwrap();
    let name = OsStr::from_bytes(b"IMG_\xff.jpg");
    let photo = dir.path().join(name);
    write(&photo, "new");
    write(&dir.path().join("Odrzucone").join(name), "old");

    let rejected = reject(&photo).unwrap();

    assert_eq!(
        rejected.path,
        dir.path()
            .join("Odrzucone")
            .join(OsStr::from_bytes(b"IMG_\xff (2).jpg"))
    );
}

const SIDECAR: &str = ".supplemental-metadata.json";

/// A photo with a Takeout date file next to it, in `dir/Album`.
fn photo_with_sidecar(dir: &TempDir) -> (PathBuf, PathBuf) {
    let photo = dir.path().join("Album/IMG_1.jpg");
    let sidecar = dir.path().join(format!("Album/IMG_1.jpg{SIDECAR}"));
    write(&photo, "photo");
    write(&sidecar, "date");
    (photo, sidecar)
}

#[test]
fn moves_the_takeout_date_file_along_and_back() {
    let dir = TempDir::new().unwrap();
    let (photo, sidecar) = photo_with_sidecar(&dir);

    let rejected = reject(&photo).unwrap();

    let moved = dir
        .path()
        .join(format!("Album/Odrzucone/IMG_1.jpg{SIDECAR}"));
    let sidecar_move = rejected.sidecar.clone().unwrap();
    assert_eq!((&sidecar_move.from, &sidecar_move.to), (&sidecar, &moved));
    assert!(!sidecar.exists());
    assert_eq!(fs::read_to_string(&moved).unwrap(), "date");

    let restored = restore(&rejected.path, &photo, rejected.sidecar.as_ref(), true).unwrap();

    assert!(restored.sidecar_error.is_none());
    assert_eq!(fs::read_to_string(&sidecar).unwrap(), "date");
    assert!(photo.exists());
    assert!(!dir.path().join("Album/Odrzucone").exists());
}

#[test]
fn a_renamed_photo_gets_a_date_file_named_after_it() {
    let dir = TempDir::new().unwrap();
    let (photo, _) = photo_with_sidecar(&dir);
    write(&dir.path().join("Album/Odrzucone/IMG_1.jpg"), "older");

    let rejected = reject(&photo).unwrap();

    let odrzucone = dir.path().join("Album/Odrzucone");
    assert_eq!(rejected.path, odrzucone.join("IMG_1 (2).jpg"));
    assert_eq!(
        rejected.sidecar.unwrap().to,
        odrzucone.join(format!("IMG_1 (2).jpg{SIDECAR}"))
    );
}

#[test]
fn if_the_photo_cannot_move_its_date_file_moves_back() {
    let dir = TempDir::new().unwrap();
    let (photo, sidecar) = photo_with_sidecar(&dir);
    let prepared = prepare(&photo).unwrap();
    // The photo disappears between preparing and moving.
    fs::rename(&photo, dir.path().join("elsewhere.jpg")).unwrap();

    assert!(complete(&photo, &prepared).is_err());

    assert_eq!(fs::read_to_string(&sidecar).unwrap(), "date");
    assert!(!dir.path().join("Album/Odrzucone").exists());
}

#[test]
fn undo_restores_the_photo_even_without_its_date_file() {
    let dir = TempDir::new().unwrap();
    let (photo, _) = photo_with_sidecar(&dir);
    let rejected = reject(&photo).unwrap();
    let sidecar_move = rejected.sidecar.clone().unwrap();
    fs::rename(&sidecar_move.to, dir.path().join("moved-away.json")).unwrap();

    let restored = restore(&rejected.path, &photo, Some(&sidecar_move), true).unwrap();

    assert!(photo.exists());
    assert!(matches!(
        restored.sidecar_error,
        Some(RejectError::Missing(_))
    ));
}

#[test]
fn an_edited_copy_leaves_the_shared_date_file_with_its_original() {
    let dir = TempDir::new().unwrap();
    let (_, sidecar) = photo_with_sidecar(&dir);
    let copy = dir.path().join("Album/IMG_1-edytowano.jpg");
    write(&copy, "edited");

    let rejected = reject(&copy).unwrap();

    assert_eq!(rejected.sidecar, None);
    assert_eq!(fs::read_to_string(&sidecar).unwrap(), "date");
}
