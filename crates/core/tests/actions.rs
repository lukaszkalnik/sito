// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use sito_core::actions::{ActionLog, LogError, LoggedAction, Placement, RejectAction};
use sito_core::reject::{Companion, FileMove, PhotoFiles};
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

/// The newest undoable action under `under`, which must be a rejection.
fn last_reject(log: &ActionLog, under: &Path) -> Result<Option<RejectAction>, LogError> {
    Ok(log.last_undoable(under)?.map(|action| match action {
        LoggedAction::Reject(reject) => reject,
        LoggedAction::Accept(accept) => panic!("not a rejection: {accept:?}"),
    }))
}

/// The other files of a photo moved into the rejected folder.
fn extra(action: RejectAction) -> Vec<FileMove> {
    match action.place {
        Placement::Folder { extra, .. } => extra,
        Placement::Trash { .. } => panic!("not a folder move: {action:?}"),
    }
}

#[test]
fn offers_the_newest_rejection_first() {
    let log = ActionLog::open_in_memory().unwrap();
    record(&log, "IMG_1.jpg");
    let newest = record(&log, "Camera/IMG_2.jpg");

    assert_eq!(
        last_reject(&log, &root()).unwrap(),
        Some(RejectAction {
            id: newest,
            from: photo("Camera/IMG_2.jpg"),
            place: Placement::Folder {
                to: rejected("Camera/IMG_2.jpg"),
                extra: Vec::new(),
            },
        })
    );
}

#[test]
fn skips_undone_rejections() {
    let log = ActionLog::open_in_memory().unwrap();
    let first = record(&log, "IMG_1.jpg");
    let second = record(&log, "IMG_2.jpg");

    log.mark_undone(second).unwrap();
    assert_eq!(last_reject(&log, &root()).unwrap().unwrap().id, first);
    assert!(log.undone_at(second).unwrap().is_some());
    assert!(log.undone_at(first).unwrap().is_none());

    log.mark_undone(first).unwrap();
    assert_eq!(last_reject(&log, &root()).unwrap(), None);
}

#[test]
fn offers_only_rejections_inside_the_given_folder() {
    let log = ActionLog::open_in_memory().unwrap();
    let inside = record(&log, "Złote gody/IMG_1.jpg");
    // Newer, but in a sibling folder whose name only starts the same.
    let sibling = root().with_file_name("Zdjęcia2").join("IMG_2.jpg");
    log.record_reject(&sibling, &sibling.with_file_name("Odrzucone"), &[])
        .unwrap();

    assert_eq!(last_reject(&log, &root()).unwrap().unwrap().id, inside);
    assert_eq!(
        last_reject(&log, &root().join("Złote gody"))
            .unwrap()
            .unwrap()
            .id,
        inside
    );
    assert_eq!(last_reject(&log, &root().join("Inne")).unwrap(), None);
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

    assert_eq!(last_reject(&log, &root()).unwrap().unwrap().id, id);
    assert_eq!(
        last_reject(&log, &root()).unwrap().unwrap().from,
        Path::new(&photo("IMG_1.jpg"))
    );
}

#[test]
fn forgets_an_action_that_did_not_happen() {
    let log = ActionLog::open_in_memory().unwrap();
    let id = record(&log, "IMG_1.jpg");

    log.forget(id).unwrap();

    assert_eq!(last_reject(&log, &root()).unwrap(), None);
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

    assert!(last_reject(&log, Path::new("a")).unwrap().is_some());
    let conn = rusqlite::Connection::open(&db).unwrap();
    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 4);
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

    let action = last_reject(&log, &root()).unwrap().unwrap();

    assert_eq!(extra(action), [sidecar]);
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

    assert_eq!(extra(last_reject(&log, &root()).unwrap().unwrap()), moves);

    log.forget(id).unwrap();
    assert_eq!(last_reject(&log, &root()).unwrap(), None);
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

    let log = ActionLog::open(&db).unwrap();
    let action = last_reject(&log, Path::new("a")).unwrap().unwrap();

    assert_eq!(
        extra(action),
        [FileMove {
            from: "a/x.jpg.json".into(),
            to: "a/Odrzucone/x.jpg.json".into(),
        }]
    );
}

#[test]
fn keeps_every_file_moved_to_the_trash_with_the_time() {
    let log = ActionLog::open_in_memory().unwrap();
    let files = PhotoFiles {
        photo: photo("P1.JPG"),
        sidecar: Some(photo("P1.JPG.supplemental-metadata.json")),
        companion: Some(Companion {
            path: photo("P1.ORF"),
            sidecar: None,
        }),
    };
    let before = chrono::Utc::now().timestamp();

    let id = log.record_trash(&files).unwrap();

    let action = last_reject(&log, &root()).unwrap().unwrap();
    assert_eq!(action.id, id);
    assert_eq!(action.from, photo("P1.JPG"));
    let Placement::Trash { files: logged, at } = action.place else {
        panic!("not a trash move: {action:?}");
    };
    assert_eq!(logged, files.all());
    assert!((before..=before + 1).contains(&at));
}

#[test]
fn offers_folder_and_trash_moves_newest_first() {
    let log = ActionLog::open_in_memory().unwrap();
    let folder = record(&log, "IMG_1.jpg");
    let trash = log
        .record_trash(&PhotoFiles {
            photo: photo("IMG_2.jpg"),
            sidecar: None,
            companion: None,
        })
        .unwrap();

    assert_eq!(last_reject(&log, &root()).unwrap().unwrap().id, trash);
    log.mark_undone(trash).unwrap();
    assert_eq!(last_reject(&log, &root()).unwrap().unwrap().id, folder);
}

#[test]
fn offers_trash_and_folder_moves_newest_first() {
    let log = ActionLog::open_in_memory().unwrap();
    let trash = log
        .record_trash(&PhotoFiles {
            photo: photo("IMG_1.jpg"),
            sidecar: None,
            companion: None,
        })
        .unwrap();
    let folder = record(&log, "IMG_2.jpg");

    assert_eq!(last_reject(&log, &root()).unwrap().unwrap().id, folder);
    log.mark_undone(folder).unwrap();
    let action = last_reject(&log, &root()).unwrap().unwrap();
    assert_eq!(action.id, trash);
    assert!(matches!(action.place, Placement::Trash { .. }));
}
