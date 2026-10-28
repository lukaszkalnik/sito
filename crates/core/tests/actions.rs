// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use sito_core::actions::{ActionLog, RejectAction};
use sito_core::reject::FileMove;
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
    log.record_reject(&photo(relative), &rejected(relative), &[])
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
            extra: Vec::new(),
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
    log.record_reject(&sibling, &sibling.with_file_name("Odrzucone"), &[])
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

#[test]
fn forgets_an_action_that_did_not_happen() {
    let log = ActionLog::open_in_memory().unwrap();
    let id = record(&log, "IMG_1.jpg");

    log.forget(id).unwrap();

    assert_eq!(log.last_undoable(&root()).unwrap(), None);
}

#[test]
fn upgrades_a_log_written_before_migrations() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("sito.db");
    // The schema as created before `user_version` was used.
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(
        "CREATE TABLE actions (
             id INTEGER PRIMARY KEY, at TEXT NOT NULL, kind TEXT NOT NULL,
             from_path TEXT NOT NULL, to_path TEXT NOT NULL, undone_at TEXT
         );
         INSERT INTO actions (at, kind, from_path, to_path)
             VALUES ('2024-06-01T12:00:00.000+02:00', 'reject', 'a', 'b');",
    )
    .unwrap();
    drop(conn);

    let log = ActionLog::open(&db).unwrap();

    assert!(log.last_undoable(Path::new("a")).unwrap().is_some());
    let conn = rusqlite::Connection::open(&db).unwrap();
    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 3);
}

#[cfg(unix)]
#[test]
fn refuses_a_path_that_is_not_unicode() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let log = ActionLog::open_in_memory().unwrap();
    let path = root().join(OsStr::from_bytes(b"IMG_\xff.jpg"));

    assert!(matches!(
        log.record_reject(&path, &rejected("IMG_1.jpg"), &[]),
        Err(sito_core::actions::LogError::NonUnicodePath(_))
    ));
}

#[test]
fn keeps_the_date_file_moved_with_a_photo() {
    let log = ActionLog::open_in_memory().unwrap();
    let sidecar = FileMove {
        from: photo("IMG_1.jpg.supplemental-metadata.json"),
        to: rejected("IMG_1.jpg.supplemental-metadata.json"),
    };
    log.record_reject(
        &photo("IMG_1.jpg"),
        &rejected("IMG_1.jpg"),
        std::slice::from_ref(&sidecar),
    )
    .unwrap();

    let action = log.last_undoable(&root()).unwrap().unwrap();

    assert_eq!(action.extra, [sidecar]);
}

#[test]
fn keeps_every_file_moved_with_a_photo_in_order() {
    let log = ActionLog::open_in_memory().unwrap();
    let moves = [
        FileMove {
            from: photo("P1.ORF.supplemental-metadata.json"),
            to: rejected("P1.ORF.supplemental-metadata.json"),
        },
        FileMove {
            from: photo("P1.ORF"),
            to: rejected("P1.ORF"),
        },
    ];
    let id = log
        .record_reject(&photo("P1.JPG"), &rejected("P1.JPG"), &moves)
        .unwrap();

    assert_eq!(log.last_undoable(&root()).unwrap().unwrap().extra, moves);

    log.forget(id).unwrap();
    assert_eq!(log.last_undoable(&root()).unwrap(), None);
}

#[test]
fn undoes_entries_logged_with_the_older_date_file_columns() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("sito.db");
    drop(ActionLog::open(&db).unwrap());
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(
        "INSERT INTO actions (at, kind, from_path, to_path, sidecar_from, sidecar_to)
         VALUES ('2026-10-28T12:00:00.000+01:00', 'reject', 'a/x.jpg', 'a/Odrzucone/x.jpg',
                 'a/x.jpg.json', 'a/Odrzucone/x.jpg.json');",
    )
    .unwrap();
    drop(conn);

    let action = ActionLog::open(&db)
        .unwrap()
        .last_undoable(Path::new("a"))
        .unwrap()
        .unwrap();

    assert_eq!(
        action.extra,
        [FileMove {
            from: "a/x.jpg.json".into(),
            to: "a/Odrzucone/x.jpg.json".into(),
        }]
    );
}
