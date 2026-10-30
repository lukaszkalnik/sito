// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::collections::HashSet;
use std::path::PathBuf;

use sito_core::accept::{AcceptAction, AcceptChange};
use sito_core::actions::{ActionLog, LoggedAction};
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

/// The newest undoable action, which must be an accept action.
fn last_accept(log: &ActionLog) -> AcceptAction {
    match log.last_undoable(&root()).unwrap() {
        Some(LoggedAction::Accept(action)) => action,
        other => panic!("not an accept action: {other:?}"),
    }
}

#[test]
fn accepting_and_undoing_marks_and_unmarks_the_photo() {
    let log = ActionLog::open_in_memory().unwrap();

    let id = log
        .set_accepted(&photo("IMG_1.jpg"), AcceptChange::Accepted)
        .unwrap()
        .unwrap();

    assert_eq!(accepted(&log), HashSet::from([photo("IMG_1.jpg")]));
    let action = last_accept(&log);
    assert_eq!(
        action,
        AcceptAction {
            id,
            path: photo("IMG_1.jpg"),
            change: AcceptChange::Accepted,
        }
    );

    log.undo_accept(&action).unwrap();

    assert_eq!(accepted(&log), HashSet::new());
    assert!(log.undone_at(id).unwrap().is_some());
    assert_eq!(log.last_undoable(&root()).unwrap(), None);
}

#[test]
fn undoing_an_unaccept_puts_the_mark_back() {
    let log = ActionLog::open_in_memory().unwrap();
    let path = photo("IMG_1.jpg");
    log.set_accepted(&path, AcceptChange::Accepted).unwrap();

    log.set_accepted(&path, AcceptChange::Unaccepted).unwrap();
    assert_eq!(accepted(&log), HashSet::new());

    let action = last_accept(&log);
    assert_eq!(action.change, AcceptChange::Unaccepted);
    log.undo_accept(&action).unwrap();
    assert_eq!(accepted(&log), HashSet::from([path.clone()]));

    // Next comes the accept itself.
    let action = last_accept(&log);
    assert_eq!(action.change, AcceptChange::Accepted);
    log.undo_accept(&action).unwrap();
    assert_eq!(accepted(&log), HashSet::new());
}

#[test]
fn a_change_that_changes_nothing_is_not_logged() {
    let log = ActionLog::open_in_memory().unwrap();
    let path = photo("IMG_1.jpg");

    assert_eq!(
        log.set_accepted(&path, AcceptChange::Unaccepted).unwrap(),
        None
    );
    assert_eq!(log.last_undoable(&root()).unwrap(), None);

    let first = log
        .set_accepted(&path, AcceptChange::Accepted)
        .unwrap()
        .unwrap();
    // Another window accepting the same photo again.
    assert_eq!(
        log.set_accepted(&path, AcceptChange::Accepted).unwrap(),
        None
    );
    assert_eq!(last_accept(&log).id, first);
}

#[test]
fn undo_takes_rejections_and_accepts_newest_first() {
    let log = ActionLog::open_in_memory().unwrap();
    let path = photo("IMG_1.jpg");
    let rejected = root().join("Odrzucone").join("IMG_1.jpg");
    log.set_accepted(&path, AcceptChange::Accepted).unwrap();
    let reject = log.record_reject(&path, &rejected, &[]).unwrap();

    let Some(LoggedAction::Reject(action)) = log.last_undoable(&root()).unwrap() else {
        panic!("the rejection comes first");
    };
    assert_eq!(action.id, reject);
    // A rejected photo keeps its mark, so undoing the rejection brings it
    // back accepted.
    assert_eq!(accepted(&log), HashSet::from([path]));
    log.mark_undone(reject).unwrap();

    assert_eq!(last_accept(&log).change, AcceptChange::Accepted);
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
    // The sibling's accept is not offered for undo in this folder.
    assert_eq!(
        log.last_undoable(&root())
            .unwrap()
            .map(|action| match action {
                LoggedAction::Accept(accept) => accept.path,
                LoggedAction::Reject(reject) => reject.from,
            }),
        Some(root().join("Złote gody").join("IMG_1.jpg"))
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
    assert_eq!(last_accept(&log).path, photo("IMG_1.jpg"));
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

    assert!(matches!(
        log.last_undoable(std::path::Path::new("a")).unwrap(),
        Some(LoggedAction::Reject(_))
    ));
    log.set_accepted(std::path::Path::new("a/y.jpg"), AcceptChange::Accepted)
        .unwrap();
    assert_eq!(
        log.accepted_paths(std::path::Path::new("a")).unwrap(),
        HashSet::from([PathBuf::from("a/y.jpg")])
    );
}
