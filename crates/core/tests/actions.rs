// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use sito_core::actions::{ActionLog, RejectAction};
use tempfile::TempDir;

fn root() -> PathBuf {
    PathBuf::from(if cfg!(windows) {
        r"D:\Zdjęcia"
    } else {
        "/home/u/Zdjęcia"
    })
}

fn photo(relative: &str) -> PathBuf {
    root().join(relative)
}

fn rejected(relative: &str) -> PathBuf {
    let path = photo(relative);
    path.parent()
        .unwrap()
        .join("Odrzucone")
        .join(path.file_name().unwrap())
}

fn record(log: &ActionLog, relative: &str) -> i64 {
    log.record_reject(&photo(relative), &rejected(relative))
        .unwrap()
}

#[test]
fn offers_the_newest_rejection_first() {
    let log = ActionLog::open_in_memory().unwrap();
    record(&log, "IMG_1.jpg");
    let newest = record(&log, "Camera/IMG_2.jpg");

    assert_eq!(
        log.last_undoable(&root()).unwrap(),
        Some(RejectAction {
            id: newest,
            from: photo("Camera/IMG_2.jpg"),
            to: rejected("Camera/IMG_2.jpg"),
        })
    );
}

#[test]
fn skips_undone_rejections() {
    let log = ActionLog::open_in_memory().unwrap();
    let first = record(&log, "IMG_1.jpg");
    let second = record(&log, "IMG_2.jpg");

    log.mark_undone(second).unwrap();
    assert_eq!(log.last_undoable(&root()).unwrap().unwrap().id, first);
    assert!(log.undone_at(second).unwrap().is_some());
    assert!(log.undone_at(first).unwrap().is_none());

    log.mark_undone(first).unwrap();
    assert_eq!(log.last_undoable(&root()).unwrap(), None);
}

#[test]
fn offers_only_rejections_inside_the_given_folder() {
    let log = ActionLog::open_in_memory().unwrap();
    let inside = record(&log, "Złote gody/IMG_1.jpg");
    // Newer, but in a sibling folder whose name only starts the same.
    let sibling = root().with_file_name("Zdjęcia2").join("IMG_2.jpg");
    log.record_reject(&sibling, &sibling.with_file_name("Odrzucone"))
        .unwrap();

    assert_eq!(log.last_undoable(&root()).unwrap().unwrap().id, inside);
    assert_eq!(
        log.last_undoable(&root().join("Złote gody"))
            .unwrap()
            .unwrap()
            .id,
        inside
    );
    assert_eq!(log.last_undoable(&root().join("Inne")).unwrap(), None);
}

#[test]
fn keeps_the_log_across_reopening() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("sito.db");
    let id = {
        let log = ActionLog::open(&file).unwrap();
        record(&log, "IMG_1.jpg")
    };

    let log = ActionLog::open(&file).unwrap();

    assert_eq!(log.last_undoable(&root()).unwrap().unwrap().id, id);
    assert_eq!(
        log.last_undoable(&root()).unwrap().unwrap().from,
        Path::new(&photo("IMG_1.jpg"))
    );
}
