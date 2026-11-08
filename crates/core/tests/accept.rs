// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use sito_core::accept::AcceptChange;
use sito_core::actions::{ActionLog, Rejections};
use tempfile::TempDir;

fn root() -> PathBuf {
    PathBuf::from(if cfg!(windows) {
        r"D:\Zdjęcia"
    } else {
        "/home/u/Zdjęcia"
    })
}

fn photo(name: &str) -> PathBuf {
    root().join(name)
}

fn accepted(log: &ActionLog) -> HashSet<PathBuf> {
    log.accepted_paths(&root()).unwrap()
}

fn rejected(name: &str) -> PathBuf {
    root().join("Odrzucone").join(name)
}

/// How many rows the `actions` table of the log file `db` holds.
fn action_rows(db: &Path) -> i64 {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn accepting_and_unaccepting_change_the_mark_without_logging() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("sito.db");
    let log = ActionLog::open(&db).unwrap();
    let path = photo("IMG_1.jpg");

    log.set_accepted(&path, AcceptChange::Accepted).unwrap();
    assert_eq!(accepted(&log), HashSet::from([path.clone()]));
    // Another window accepting the same photo again changes nothing.
    log.set_accepted(&path, AcceptChange::Accepted).unwrap();
    assert_eq!(accepted(&log), HashSet::from([path.clone()]));

    log.set_accepted(&path, AcceptChange::Unaccepted).unwrap();
    assert_eq!(accepted(&log), HashSet::new());

    assert_eq!(log.last_undoable(&root(), Rejections::All).unwrap(), None);
    assert_eq!(action_rows(&db), 0);
}

#[test]
fn undo_skips_accepts_and_keeps_their_marks() {
    let log = ActionLog::open_in_memory().unwrap();
    log.set_accepted(&photo("IMG_1.jpg"), AcceptChange::Accepted)
        .unwrap();
    let reject = log
        .record_reject(&photo("IMG_1.jpg"), &rejected("IMG_1.jpg"), &[])
        .unwrap();
    log.set_accepted(&photo("IMG_2.jpg"), AcceptChange::Accepted)
        .unwrap();

    let action = log
        .last_undoable(&root(), Rejections::All)
        .unwrap()
        .unwrap();
    assert_eq!(action.id, reject);
    log.mark_undone(reject).unwrap();

    assert_eq!(log.last_undoable(&root(), Rejections::All).unwrap(), None);
    // A rejected accepted photo keeps its mark, so it comes back accepted.
    assert_eq!(
        accepted(&log),
        HashSet::from([photo("IMG_1.jpg"), photo("IMG_2.jpg")])
    );
}

#[test]
fn old_logged_accepts_are_not_offered_for_undo() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("sito.db");
    let log = ActionLog::open(&db).unwrap();
    let reject = log
        .record_reject(&photo("IMG_1.jpg"), &rejected("IMG_1.jpg"), &[])
        .unwrap();
    // Rows written by versions that still logged accepting (slices 32–42).
    let text = photo("IMG_2.jpg").to_str().unwrap().to_owned();
    let conn = rusqlite::Connection::open(&db).unwrap();
    for kind in ["accept", "unaccept"] {
        conn.execute(
            "INSERT INTO actions (at, kind, from_path, to_path)
             VALUES ('2026-11-01T12:00:00.000+01:00', ?1, ?2, ?2)",
            rusqlite::params![kind, text],
        )
        .unwrap();
    }

    assert_eq!(
        log.last_undoable(&root(), Rejections::All)
            .unwrap()
            .unwrap()
            .id,
        reject
    );
    log.mark_undone(reject).unwrap();
    assert_eq!(log.last_undoable(&root(), Rejections::All).unwrap(), None);
}

#[test]
fn lists_only_accepted_photos_inside_the_folder() {
    let log = ActionLog::open_in_memory().unwrap();
    let inside = root().join("Złote gody").join("IMG_1.jpg");
    // In a sibling folder whose name only starts the same.
    let sibling = root().with_file_name("Zdjęcia2").join("IMG_2.jpg");
    log.set_accepted(&inside, AcceptChange::Accepted).unwrap();
    log.set_accepted(&sibling, AcceptChange::Accepted).unwrap();

    assert_eq!(accepted(&log), HashSet::from([inside.clone()]));
    assert_eq!(
        log.accepted_paths(&root().join("Złote gody")).unwrap(),
        HashSet::from([inside])
    );
}

#[test]
fn keeps_the_marks_across_reopening() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("sito.db");
    {
        let log = ActionLog::open(&file).unwrap();
        log.set_accepted(&photo("IMG_1.jpg"), AcceptChange::Accepted)
            .unwrap();
    }

    let log = ActionLog::open(&file).unwrap();

    assert_eq!(accepted(&log), HashSet::from([photo("IMG_1.jpg")]));
}

#[test]
fn upgrading_a_version_3_log_keeps_its_entries() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("sito.db");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(
        "CREATE TABLE actions (
             id INTEGER PRIMARY KEY, at TEXT NOT NULL, kind TEXT NOT NULL,
             from_path TEXT NOT NULL, to_path TEXT NOT NULL, undone_at TEXT,
             sidecar_from TEXT, sidecar_to TEXT
         );
         CREATE TABLE action_moves (
             action_id INTEGER NOT NULL REFERENCES actions(id),
             from_path TEXT NOT NULL, to_path TEXT NOT NULL
         );
         INSERT INTO actions (at, kind, from_path, to_path)
             VALUES ('2026-10-28T12:00:00.000+01:00', 'reject', 'a/x.jpg', 'a/Odrzucone/x.jpg');
         PRAGMA user_version = 3;",
    )
    .unwrap();
    drop(conn);

    let log = ActionLog::open(&db).unwrap();

    assert!(
        log.last_undoable(Path::new("a"), Rejections::All)
            .unwrap()
            .is_some()
    );
    log.set_accepted(Path::new("a/y.jpg"), AcceptChange::Accepted)
        .unwrap();
    assert_eq!(
        log.accepted_paths(Path::new("a")).unwrap(),
        HashSet::from([PathBuf::from("a/y.jpg")])
    );
}
