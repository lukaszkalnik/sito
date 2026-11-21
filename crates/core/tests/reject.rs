// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};

use sito_core::reject::{PhotoFiles, RejectError, abandon, complete, prepare, reject, restore};
use sito_core::scan::RejectedDirName;
use tempfile::TempDir;

/// The folder name these tests reject into.
const ODRZUCONE: RejectedDirName = RejectedDirName::ODRZUCONE;

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn moves_the_photo_into_odrzucone_next_to_it() {
    let dir = TempDir::new().unwrap();

    let photo = dir.path().join("Camera/IMG_1.jpg");
    write(&photo, "photo");

    let rejected = reject(&photo, ODRZUCONE).unwrap();

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

    assert!(!reject(&photo, ODRZUCONE).unwrap().created_dir);
}

#[test]
fn never_overwrites_an_earlier_rejected_photo() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&dir.path().join("Odrzucone/IMG_1.jpg"), "older");
    write(&dir.path().join("Odrzucone/IMG_1 (2).jpg"), "old");
    write(&photo, "new");

    let rejected = reject(&photo, ODRZUCONE).unwrap();

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
    let rejected = reject(&photo, ODRZUCONE).unwrap();

    restore(&rejected.path, &photo, &[], true).unwrap();

    assert_eq!(fs::read_to_string(&photo).unwrap(), "photo");
    assert!(!dir.path().join("Odrzucone").exists());
}

#[test]
fn keeps_an_empty_folder_that_is_not_to_be_removed() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "photo");
    let rejected = reject(&photo, ODRZUCONE).unwrap();

    restore(&rejected.path, &photo, &[], false).unwrap();

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
    let rejected_first = reject(&first, ODRZUCONE).unwrap();
    reject(&second, ODRZUCONE).unwrap();

    restore(&rejected_first.path, &first, &[], true).unwrap();

    assert!(first.exists());
    assert!(dir.path().join("Odrzucone/IMG_2.jpg").exists());
}

#[test]
fn restores_a_renamed_photo_under_its_original_name() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&dir.path().join("Odrzucone/IMG_1.jpg"), "other");
    write(&photo, "mine");
    let rejected = reject(&photo, ODRZUCONE).unwrap();

    restore(&rejected.path, &photo, &[], true).unwrap();

    assert_eq!(fs::read_to_string(&photo).unwrap(), "mine");
    assert_eq!(
        fs::read_to_string(dir.path().join("Odrzucone/IMG_1.jpg")).unwrap(),
        "other"
    );
}

#[test]
fn restores_under_a_free_name_when_the_original_name_is_taken() {
    let dir = TempDir::new().unwrap();
    let (photo, sidecar) = photo_with_sidecar(&dir);
    let rejected = reject(&photo, ODRZUCONE).unwrap();
    write(&photo, "new file");

    let restored = restore(&rejected.path, &photo, &rejected.extra, true).unwrap();

    let album = dir.path().join("Album");
    assert_eq!(restored.path, album.join("IMG_1 (2).jpg"));
    assert!(restored.errors.is_empty());
    assert_eq!(fs::read_to_string(&restored.path).unwrap(), "photo");
    // The file that took the name is untouched; the date file follows the new name.
    assert_eq!(fs::read_to_string(&photo).unwrap(), "new file");
    assert!(!sidecar.exists());
    assert_eq!(
        fs::read_to_string(album.join(format!("IMG_1 (2).jpg{SIDECAR}"))).unwrap(),
        "date"
    );
}

#[test]
fn a_pair_with_one_name_taken_goes_back_with_one_number() {
    let dir = TempDir::new().unwrap();
    let (jpeg, raw) = pair_with_sidecars(&dir);
    let rejected = reject(&jpeg, ODRZUCONE).unwrap();
    write(&raw, "another raw");

    let restored = restore(&rejected.path, &jpeg, &rejected.extra, true).unwrap();

    let album = dir.path().join("Album");
    assert_eq!(restored.path, album.join("P1010001 (2).JPG"));
    assert!(restored.errors.is_empty());
    assert_eq!(
        fs::read_to_string(album.join("P1010001 (2).ORF")).unwrap(),
        "raw"
    );
    assert_eq!(fs::read_to_string(&raw).unwrap(), "another raw");
    assert!(!jpeg.exists());
    for (name, content) in [
        ("P1010001 (2).JPG", "jpeg date"),
        ("P1010001 (2).ORF", "raw date"),
    ] {
        let sidecar = album.join(format!("{name}{SIDECAR}"));
        assert_eq!(fs::read_to_string(sidecar).unwrap(), content);
    }
    assert!(!album.join("Odrzucone").exists());
}

#[test]
fn reports_a_rejected_photo_that_is_gone() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    let gone = dir.path().join("Odrzucone/IMG_1.jpg");

    let err = restore(&gone, &photo, &[], true).unwrap_err();

    assert!(matches!(err, RejectError::Missing(path) if path == gone));
}

#[test]
fn reports_a_photo_that_is_gone_before_preparing_anything() {
    let dir = TempDir::new().unwrap();
    let gone = dir.path().join("missing.jpg");

    let err = prepare(&gone, ODRZUCONE).unwrap_err();

    assert!(matches!(err, RejectError::PhotoGone(path) if path == gone));
    assert!(!dir.path().join("Odrzucone").exists());
}

#[test]
fn prepare_picks_the_target_without_moving() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "photo");

    let prepared = prepare(&photo, ODRZUCONE).unwrap();

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
    let prepared = prepare(&photo, ODRZUCONE).unwrap();
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

    abandon(&prepare(&photo, ODRZUCONE).unwrap());
    assert!(!dir.path().join("Odrzucone").exists());

    fs::create_dir(dir.path().join("Odrzucone")).unwrap();
    abandon(&prepare(&photo, ODRZUCONE).unwrap());
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

    let rejected = reject(&photo, ODRZUCONE).unwrap();

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

    let rejected = reject(&photo, ODRZUCONE).unwrap();

    let moved = dir
        .path()
        .join(format!("Album/Odrzucone/IMG_1.jpg{SIDECAR}"));
    let sidecar_move = rejected.extra[0].clone();
    assert_eq!((&sidecar_move.from, &sidecar_move.to), (&sidecar, &moved));
    assert!(!sidecar.exists());
    assert_eq!(fs::read_to_string(&moved).unwrap(), "date");

    let restored = restore(&rejected.path, &photo, &rejected.extra, true).unwrap();

    assert!(restored.errors.is_empty());
    assert_eq!(fs::read_to_string(&sidecar).unwrap(), "date");
    assert!(photo.exists());
    assert!(!dir.path().join("Album/Odrzucone").exists());
}

#[test]
fn a_renamed_photo_gets_a_date_file_named_after_it() {
    let dir = TempDir::new().unwrap();
    let (photo, _) = photo_with_sidecar(&dir);
    write(&dir.path().join("Album/Odrzucone/IMG_1.jpg"), "older");

    let rejected = reject(&photo, ODRZUCONE).unwrap();

    let odrzucone = dir.path().join("Album/Odrzucone");
    assert_eq!(rejected.path, odrzucone.join("IMG_1 (2).jpg"));
    assert_eq!(
        rejected.extra[0].to,
        odrzucone.join(format!("IMG_1 (2).jpg{SIDECAR}"))
    );
}

#[test]
fn if_the_photo_cannot_move_its_date_file_moves_back() {
    let dir = TempDir::new().unwrap();
    let (photo, sidecar) = photo_with_sidecar(&dir);
    let prepared = prepare(&photo, ODRZUCONE).unwrap();
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
    let rejected = reject(&photo, ODRZUCONE).unwrap();
    let sidecar_move = rejected.extra[0].clone();
    fs::rename(&sidecar_move.to, dir.path().join("moved-away.json")).unwrap();

    let restored = restore(
        &rejected.path,
        &photo,
        std::slice::from_ref(&sidecar_move),
        true,
    )
    .unwrap();

    assert!(photo.exists());
    assert!(matches!(
        restored.errors.as_slice(),
        [RejectError::Missing(_)]
    ));
}

#[test]
fn an_edited_copy_leaves_the_shared_date_file_with_its_original() {
    let dir = TempDir::new().unwrap();
    let (_, sidecar) = photo_with_sidecar(&dir);
    let copy = dir.path().join("Album/IMG_1-edytowano.jpg");
    write(&copy, "edited");

    let rejected = reject(&copy, ODRZUCONE).unwrap();

    assert_eq!(rejected.extra, []);
    assert_eq!(fs::read_to_string(&sidecar).unwrap(), "date");
}

/// A RAW+JPEG pair with Takeout date files for both, in `dir/Album`.
fn pair_with_sidecars(dir: &TempDir) -> (PathBuf, PathBuf) {
    let jpeg = dir.path().join("Album/P1010001.JPG");
    let raw = dir.path().join("Album/P1010001.ORF");
    write(&jpeg, "jpeg");
    write(&raw, "raw");
    write(
        &dir.path().join(format!("Album/P1010001.JPG{SIDECAR}")),
        "jpeg date",
    );
    write(
        &dir.path().join(format!("Album/P1010001.ORF{SIDECAR}")),
        "raw date",
    );
    (jpeg, raw)
}

#[test]
fn rejects_and_restores_every_file_of_a_raw_jpeg_pair() {
    let dir = TempDir::new().unwrap();
    let (jpeg, raw) = pair_with_sidecars(&dir);
    let album = dir.path().join("Album");

    let rejected = reject(&jpeg, ODRZUCONE).unwrap();

    let odrzucone = album.join("Odrzucone");
    let mut there: Vec<String> = fs::read_dir(&odrzucone)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    there.sort();
    assert_eq!(
        there,
        [
            "P1010001.JPG",
            "P1010001.JPG.supplemental-metadata.json",
            "P1010001.ORF",
            "P1010001.ORF.supplemental-metadata.json",
        ]
    );
    assert!(!raw.exists());

    let restored = restore(&rejected.path, &jpeg, &rejected.extra, true).unwrap();

    assert!(restored.errors.is_empty());
    assert_eq!(fs::read_to_string(&raw).unwrap(), "raw");
    assert_eq!(
        fs::read_to_string(album.join(format!("P1010001.ORF{SIDECAR}"))).unwrap(),
        "raw date"
    );
    assert!(!odrzucone.exists());
}

#[test]
fn a_pair_gets_one_common_number_when_its_names_are_taken() {
    let dir = TempDir::new().unwrap();
    let (jpeg, _) = pair_with_sidecars(&dir);
    write(
        &dir.path().join("Album/Odrzucone/P1010001.ORF"),
        "older raw",
    );

    let rejected = reject(&jpeg, ODRZUCONE).unwrap();

    let odrzucone = dir.path().join("Album/Odrzucone");
    assert_eq!(rejected.path, odrzucone.join("P1010001 (2).JPG"));
    assert!(odrzucone.join("P1010001 (2).ORF").is_file());
    assert!(
        odrzucone
            .join(format!("P1010001 (2).ORF{SIDECAR}"))
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(odrzucone.join("P1010001.ORF")).unwrap(),
        "older raw"
    );
}

#[test]
fn photo_files_of_a_pair_lists_the_photo_first_then_its_other_files() {
    let dir = TempDir::new().unwrap();
    let (jpeg, raw) = pair_with_sidecars(&dir);
    let album = dir.path().join("Album");

    assert_eq!(
        PhotoFiles::find(&jpeg).all(),
        [
            jpeg,
            album.join(format!("P1010001.JPG{SIDECAR}")),
            raw,
            album.join(format!("P1010001.ORF{SIDECAR}")),
        ]
    );
}

#[test]
fn photo_files_of_a_lone_photo_is_the_photo_alone() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "photo");

    assert_eq!(PhotoFiles::find(&photo).all(), [photo]);
}

#[test]
fn photo_files_leave_out_a_date_file_shared_by_an_original_and_its_edited_copy() {
    let dir = TempDir::new().unwrap();
    let (photo, _) = photo_with_sidecar(&dir);
    let copy = dir.path().join("Album/IMG_1-edytowano.jpg");
    write(&copy, "edited");

    assert_eq!(PhotoFiles::find(&copy).all(), [copy]);
    assert_eq!(PhotoFiles::find(&photo).all(), [photo]);
}

#[cfg(unix)]
#[test]
fn refuses_an_odrzucone_that_is_a_link() {
    let dir = TempDir::new().unwrap();
    let elsewhere = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "photo");
    let link = dir.path().join("Odrzucone");
    std::os::unix::fs::symlink(elsewhere.path(), &link).unwrap();

    let err = reject(&photo, ODRZUCONE).unwrap_err();

    assert!(matches!(err, RejectError::RejectedDirIsLink(path) if path == link));
    assert_eq!(fs::read_to_string(&photo).unwrap(), "photo");
    assert_eq!(fs::read_dir(elsewhere.path()).unwrap().count(), 0);
}

/// A photo `IMG_1.jpg` in a temp folder that already has the folders `dirs`.
fn photo_next_to(dirs: &[&str]) -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    for name in dirs {
        fs::create_dir(dir.path().join(name)).unwrap();
    }
    let photo = dir.path().join("IMG_1.jpg");
    write(&photo, "photo");
    (dir, photo)
}

/// The folders in `dir`, by name.
fn folders(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_folder_without_a_rejected_folder_gets_one_of_the_given_name() {
    for name in RejectedDirName::ALL {
        let (dir, photo) = photo_next_to(&[]);

        let rejected = reject(&photo, name).unwrap();

        assert_eq!(
            rejected.path,
            dir.path().join(name.as_str()).join("IMG_1.jpg")
        );
        assert!(rejected.created_dir);
    }
}

#[test]
fn an_existing_rejected_folder_of_any_name_is_used() {
    for existing in RejectedDirName::ALL {
        for wanted in RejectedDirName::ALL {
            let (dir, photo) = photo_next_to(&[existing.as_str()]);

            let rejected = reject(&photo, wanted).unwrap();

            let into = dir.path().join(existing.as_str());
            assert_eq!(rejected.path, into.join("IMG_1.jpg"), "{wanted:?}");
            assert!(!rejected.created_dir);
            assert_eq!(folders(dir.path()), [existing.as_str()]);
        }
    }
}

#[test]
fn of_several_rejected_folders_the_given_name_wins() {
    let (dir, photo) = photo_next_to(&["Abgelehnt", "Odrzucone", "Rejected"]);

    let rejected = reject(&photo, RejectedDirName::REJECTED).unwrap();

    assert_eq!(rejected.path, dir.path().join("Rejected/IMG_1.jpg"));
}

#[test]
fn of_several_other_rejected_folders_the_first_by_name_is_used() {
    let (dir, photo) = photo_next_to(&["Rejected", "Odrzucone"]);

    let rejected = reject(&photo, RejectedDirName::ABGELEHNT).unwrap();

    assert_eq!(rejected.path, dir.path().join("Odrzucone/IMG_1.jpg"));
    assert_eq!(folders(dir.path()), ["Odrzucone", "Rejected"]);
}

#[test]
fn a_file_with_a_rejected_folder_name_is_no_rejected_folder() {
    let (dir, photo) = photo_next_to(&["Odrzucone"]);
    write(&dir.path().join("Rejected"), "not a folder");

    let rejected = reject(&photo, RejectedDirName::REJECTED).unwrap();

    assert_eq!(rejected.path, dir.path().join("Odrzucone/IMG_1.jpg"));
}

// Windows and macOS find a folder in any letter case; Linux only by its
// exact name.
#[cfg(any(windows, target_os = "macos"))]
#[test]
fn a_rejected_folder_in_another_letter_case_is_used() {
    let (dir, photo) = photo_next_to(&["odrzucone"]);

    let rejected = reject(&photo, RejectedDirName::REJECTED).unwrap();

    assert!(!rejected.created_dir);
    assert_eq!(folders(dir.path()), ["odrzucone"]);
}

#[test]
fn each_folder_has_its_own_rejected_folder() {
    let (dir, _) = photo_next_to(&["Odrzucone"]);
    let nested = dir.path().join("Camera/IMG_2.jpg");
    write(&nested, "nested");

    let rejected = reject(&nested, RejectedDirName::ABGELEHNT).unwrap();

    assert_eq!(rejected.path, dir.path().join("Camera/Abgelehnt/IMG_2.jpg"));
    assert!(rejected.created_dir);
}

#[test]
fn undo_from_a_reused_folder_keeps_the_folder() {
    let (dir, photo) = photo_next_to(&["Odrzucone"]);
    write(&dir.path().join("Odrzucone/IMG_0.jpg"), "earlier");

    let rejected = reject(&photo, RejectedDirName::REJECTED).unwrap();
    restore(
        &rejected.path,
        &photo,
        &rejected.extra,
        rejected.created_dir,
    )
    .unwrap();

    assert_eq!(fs::read_to_string(&photo).unwrap(), "photo");
    assert_eq!(folders(dir.path()), ["Odrzucone"]);
    assert!(dir.path().join("Odrzucone/IMG_0.jpg").exists());
}

#[cfg(unix)]
#[test]
fn a_link_with_the_given_name_is_refused_even_next_to_a_rejected_folder() {
    let elsewhere = TempDir::new().unwrap();
    let (dir, photo) = photo_next_to(&["Odrzucone"]);
    let link = dir.path().join("Rejected");
    std::os::unix::fs::symlink(elsewhere.path(), &link).unwrap();

    let err = reject(&photo, RejectedDirName::REJECTED).unwrap_err();

    assert!(matches!(err, RejectError::RejectedDirIsLink(path) if path == link));
    assert_eq!(fs::read_to_string(&photo).unwrap(), "photo");
}

#[cfg(unix)]
#[test]
fn a_link_of_another_name_loses_to_a_folder_first_by_name() {
    let elsewhere = TempDir::new().unwrap();
    let (dir, photo) = photo_next_to(&["Odrzucone"]);
    std::os::unix::fs::symlink(elsewhere.path(), dir.path().join("Rejected")).unwrap();

    let rejected = reject(&photo, RejectedDirName::ABGELEHNT).unwrap();

    assert_eq!(rejected.path, dir.path().join("Odrzucone/IMG_1.jpg"));
    assert_eq!(fs::read_dir(elsewhere.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn a_link_first_by_name_is_refused() {
    let elsewhere = TempDir::new().unwrap();
    let (dir, photo) = photo_next_to(&["Rejected"]);
    let link = dir.path().join("Abgelehnt");
    std::os::unix::fs::symlink(elsewhere.path(), &link).unwrap();

    let err = reject(&photo, RejectedDirName::ODRZUCONE).unwrap_err();

    assert!(matches!(err, RejectError::RejectedDirIsLink(path) if path == link));
}
