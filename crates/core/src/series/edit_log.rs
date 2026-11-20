//! Series edits in the action log: each edit is an action with the pins its
//! photos had before it, so „Cofnij” can put them back, newest first and in
//! one history with the rejections.

use std::path::{Path, PathBuf};

use rusqlite::params;

use super::edit::EditKind;
use super::pin_store::{pin_of, series_of, stored_pin, write_pin_changes};
use super::pins::{Pin, PinChanges};
use crate::actions::{ActionLog, LogError, now, path_text};

/// A series edit that has not been undone yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditAction {
    /// Row id in the action log; identifies the action to undo.
    pub id: i64,
    /// Which edit it was.
    pub kind: EditKind,
    /// The photo the edit was about (see [`Edit::photo`](super::Edit::photo)),
    /// at the path it had then.
    pub photo: PathBuf,
}

const KIND_CUT: &str = "series_cut";
const KIND_REMOVE: &str = "series_remove";
const KIND_JOIN: &str = "series_join";
const KIND_UNGROUP: &str = "series_ungroup";
const KIND_RESTORE: &str = "series_restore";

/// How the log names `kind`.
fn kind_text(kind: EditKind) -> &'static str {
    match kind {
        EditKind::Cut => KIND_CUT,
        EditKind::Remove => KIND_REMOVE,
        EditKind::Join => KIND_JOIN,
        EditKind::Ungroup => KIND_UNGROUP,
        EditKind::Restore => KIND_RESTORE,
    }
}

/// The edit the log names `text`, if it names one.
#[must_use]
pub(crate) fn edit_kind_of(text: &str) -> Option<EditKind> {
    match text {
        KIND_CUT => Some(EditKind::Cut),
        KIND_REMOVE => Some(EditKind::Remove),
        KIND_JOIN => Some(EditKind::Join),
        KIND_UNGROUP => Some(EditKind::Ungroup),
        KIND_RESTORE => Some(EditKind::Restore),
        _ => None,
    }
}

/// The pin a photo has after `changes`, given the one it had before
/// (`changes` clear first, then set).
fn pin_after(changes: &PinChanges, path: &Path, before: Option<Pin>) -> Option<Pin> {
    if let Some((_, pin)) = changes.set.iter().rev().find(|(p, _)| p == path) {
        return Some(*pin);
    }
    if changes.clear.iter().any(|p| p == path) {
        return None;
    }
    before
}

/// The paths `changes` touch, each once, in the order they first appear.
fn touched(changes: &PinChanges) -> Vec<&Path> {
    let mut paths: Vec<&Path> = Vec::new();
    for path in changes
        .clear
        .iter()
        .chain(changes.set.iter().map(|(path, _)| path))
    {
        if !paths.contains(&path.as_path()) {
            paths.push(path);
        }
    }
    paths
}

impl ActionLog {
    /// Applies the pin `changes` of edit `kind` about `photo` and logs the
    /// edit with the pins every touched photo had before, all in one
    /// transaction; returns the action id. An edit that changes no pin is
    /// not logged (`None`): there would be nothing to undo.
    ///
    /// # Errors
    ///
    /// Returns an error if a path is not valid Unicode or the database
    /// cannot be written; then nothing changes.
    pub fn record_edit(
        &self,
        kind: EditKind,
        photo: &Path,
        changes: &PinChanges,
    ) -> Result<Option<i64>, LogError> {
        let transaction = self.conn.unchecked_transaction()?;
        let mut previous = Vec::new();
        for path in touched(changes) {
            let before = stored_pin(&transaction, path)?;
            if pin_after(changes, path, before) != before {
                previous.push((path, before));
            }
        }
        if previous.is_empty() {
            return Ok(None);
        }
        let photo = path_text(photo)?;
        transaction.execute(
            "INSERT INTO actions (at, kind, from_path, to_path) VALUES (?1, ?2, ?3, ?3)",
            params![now(), kind_text(kind), photo],
        )?;
        let id = transaction.last_insert_rowid();
        for (path, before) in previous {
            transaction.execute(
                "INSERT INTO action_pins (action_id, path, pinned, series) VALUES (?1, ?2, ?3, ?4)",
                params![
                    id,
                    path_text(path)?,
                    before.is_some(),
                    before.and_then(series_of)
                ],
            )?;
        }
        write_pin_changes(&transaction, changes)?;
        transaction.commit()?;
        Ok(Some(id))
    }

    /// Undoes the edit `id`: puts back the pins its photos had before it
    /// and marks it undone, in one transaction. Returns the pin changes this
    /// made, for the open folder to apply.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read or written; then
    /// nothing changes.
    pub fn undo_edit(&self, id: i64) -> Result<PinChanges, LogError> {
        let transaction = self.conn.unchecked_transaction()?;
        let changes = {
            let mut statement = transaction.prepare(
                "SELECT path, pinned, series FROM action_pins WHERE action_id = ?1 ORDER BY rowid",
            )?;
            let rows = statement.query_map(params![id], |row| {
                Ok((
                    PathBuf::from(row.get::<_, String>(0)?),
                    row.get::<_, bool>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            })?;
            let mut changes = PinChanges::default();
            for row in rows {
                let (path, pinned, series) = row?;
                if pinned {
                    changes.set.push((path, pin_of(series)));
                } else {
                    changes.clear.push(path);
                }
            }
            changes
        };
        write_pin_changes(&transaction, &changes)?;
        transaction.execute(
            "UPDATE actions SET undone_at = ?1 WHERE id = ?2",
            params![now(), id],
        )?;
        transaction.commit()?;
        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Rejections, Undoable};
    use crate::moved::MovedPhoto;
    use crate::series::ManualSeriesId;

    fn log() -> ActionLog {
        ActionLog::open_in_memory().unwrap()
    }

    fn manual(id: i64) -> Pin {
        Pin::Series(ManualSeriesId(id))
    }

    fn set(pins: &[(&str, Pin)]) -> PinChanges {
        PinChanges {
            set: pins
                .iter()
                .map(|&(p, pin)| (PathBuf::from(p), pin))
                .collect(),
            clear: Vec::new(),
        }
    }

    fn pin(log: &ActionLog, path: &str) -> Option<Pin> {
        log.pins_under(Path::new("")).unwrap().get(Path::new(path))
    }

    fn last_edit(log: &ActionLog, under: &str) -> Option<EditAction> {
        match log
            .last_undoable(Path::new(under), Rejections::All)
            .unwrap()
        {
            Some(Undoable::Edit(edit)) => Some(edit),
            Some(Undoable::Reject(action)) => panic!("not an edit: {action:?}"),
            None => None,
        }
    }

    const KINDS: [EditKind; 5] = [
        EditKind::Cut,
        EditKind::Remove,
        EditKind::Join,
        EditKind::Ungroup,
        EditKind::Restore,
    ];

    #[test]
    fn every_kind_round_trips_through_its_text() {
        for kind in KINDS {
            assert_eq!(edit_kind_of(kind_text(kind)), Some(kind));
        }
        assert_eq!(edit_kind_of("reject"), None);
        assert_eq!(edit_kind_of("accept"), None);
    }

    #[test]
    fn every_kind_is_logged_and_offered_by_undo() {
        let log = log();
        for (number, kind) in KINDS.into_iter().enumerate() {
            let photo = format!("a/{number}.jpg");
            let id = log
                .record_edit(kind, Path::new(&photo), &set(&[(&photo, Pin::Single)]))
                .unwrap()
                .unwrap();

            assert_eq!(
                last_edit(&log, "a"),
                Some(EditAction {
                    id,
                    kind,
                    photo: PathBuf::from(photo),
                })
            );
        }
    }

    #[test]
    fn undo_puts_back_every_kind_of_previous_pin() {
        let log = log();
        log.apply_pin_changes(&set(&[
            ("single.jpg", Pin::Single),
            ("other.jpg", manual(3)),
        ]))
        .unwrap();
        let changes = PinChanges {
            set: vec![
                (PathBuf::from("none.jpg"), manual(7)),
                (PathBuf::from("single.jpg"), manual(7)),
            ],
            clear: vec![PathBuf::from("other.jpg")],
        };
        let id = log
            .record_edit(EditKind::Join, Path::new("none.jpg"), &changes)
            .unwrap()
            .unwrap();
        assert_eq!(pin(&log, "none.jpg"), Some(manual(7)));
        assert_eq!(pin(&log, "other.jpg"), None);

        let undone = log.undo_edit(id).unwrap();

        assert_eq!(pin(&log, "none.jpg"), None);
        assert_eq!(pin(&log, "single.jpg"), Some(Pin::Single));
        assert_eq!(pin(&log, "other.jpg"), Some(manual(3)));
        assert_eq!(undone.clear, [PathBuf::from("none.jpg")]);
        assert_eq!(undone.set.len(), 2);
        assert!(log.undone_at(id).unwrap().is_some());
        assert_eq!(last_edit(&log, ""), None);
    }

    #[test]
    fn an_edit_that_changes_no_pin_is_not_logged() {
        let log = log();
        log.apply_pin_changes(&set(&[("x.jpg", Pin::Single)]))
            .unwrap();

        let unchanged = set(&[("x.jpg", Pin::Single)]);
        let nothing_to_clear = PinChanges {
            set: Vec::new(),
            clear: vec![PathBuf::from("y.jpg")],
        };

        for changes in [unchanged, nothing_to_clear, PinChanges::default()] {
            assert_eq!(
                log.record_edit(EditKind::Remove, Path::new("x.jpg"), &changes)
                    .unwrap(),
                None
            );
        }
        assert_eq!(last_edit(&log, ""), None);
    }

    #[test]
    fn only_the_pins_that_change_are_logged() {
        let log = log();
        log.apply_pin_changes(&set(&[("a.jpg", manual(1)), ("b.jpg", manual(1))]))
            .unwrap();
        // „Wyjmij z serii” of b: a keeps its pin, b becomes single.
        let id = log
            .record_edit(
                EditKind::Remove,
                Path::new("b.jpg"),
                &set(&[("a.jpg", manual(1)), ("b.jpg", Pin::Single)]),
            )
            .unwrap()
            .unwrap();

        let undone = log.undo_edit(id).unwrap();

        assert_eq!(undone.set, [(PathBuf::from("b.jpg"), manual(1))]);
        assert_eq!(undone.clear, Vec::<PathBuf>::new());
    }

    #[test]
    fn edits_and_rejections_are_undone_newest_first() {
        let log = log();
        let reject = |name: &str| {
            log.record_reject(Path::new(name), &Path::new("Odrzucone").join(name), &[])
                .unwrap()
        };
        let edit = |name: &str| {
            log.record_edit(
                EditKind::Remove,
                Path::new(name),
                &set(&[(name, Pin::Single)]),
            )
            .unwrap()
            .unwrap()
        };
        let first = edit("1.jpg");
        let second = reject("2.jpg");
        let third = edit("3.jpg");
        let ids = || {
            log.last_undoable(Path::new(""), Rejections::All)
                .unwrap()
                .map(|undoable| match undoable {
                    Undoable::Reject(action) => action.id,
                    Undoable::Edit(edit) => edit.id,
                })
        };

        assert_eq!(ids(), Some(third));
        log.undo_edit(third).unwrap();
        assert_eq!(ids(), Some(second));
        log.mark_undone(second).unwrap();
        assert_eq!(ids(), Some(first));
        log.undo_edit(first).unwrap();
        assert_eq!(ids(), None);
    }

    #[test]
    fn undo_offers_edits_of_the_folder_only() {
        let log = log();
        log.record_edit(
            EditKind::Cut,
            Path::new("a/x.jpg"),
            &set(&[("a/x.jpg", Pin::Single)]),
        )
        .unwrap();

        assert!(last_edit(&log, "a").is_some());
        assert_eq!(last_edit(&log, "a2"), None);
        assert_eq!(last_edit(&log, "b"), None);
    }

    #[test]
    fn an_edit_behind_a_skipped_trash_move_is_offered() {
        let log = log();
        let id = log
            .record_edit(
                EditKind::Remove,
                Path::new("x.jpg"),
                &set(&[("x.jpg", Pin::Single)]),
            )
            .unwrap()
            .unwrap();
        log.record_trash(&crate::reject::PhotoFiles {
            photo: PathBuf::from("y.jpg"),
            sidecar: None,
            companion: None,
        })
        .unwrap();

        let offered = log
            .last_undoable(Path::new(""), Rejections::FolderOnly)
            .unwrap();

        assert!(matches!(offered, Some(Undoable::Edit(edit)) if edit.id == id));
    }

    #[test]
    fn previous_pins_follow_photos_moved_while_their_edit_is_open() {
        let log = log();
        let open = log
            .record_edit(
                EditKind::Remove,
                Path::new("x.jpg"),
                &set(&[("x.jpg", Pin::Single), ("y.jpg", manual(2))]),
            )
            .unwrap()
            .unwrap();
        let undone = log
            .record_edit(
                EditKind::Join,
                Path::new("y.jpg"),
                &set(&[("y.jpg", manual(4))]),
            )
            .unwrap()
            .unwrap();
        log.undo_edit(undone).unwrap();

        log.move_pins(&[MovedPhoto {
            from: PathBuf::from("y.jpg"),
            to: PathBuf::from("z.jpg"),
        }])
        .unwrap();
        let changes = log.undo_edit(open).unwrap();

        // The open edit's previous pin of y (none) went with it to z.
        assert_eq!(
            changes.clear,
            [PathBuf::from("x.jpg"), PathBuf::from("z.jpg")]
        );
        assert_eq!(pin(&log, "z.jpg"), None);
        // The undone edit's row keeps its old path.
        let rows: i64 = log
            .conn
            .query_row(
                "SELECT COUNT(*) FROM action_pins WHERE action_id = ?1 AND path = 'y.jpg'",
                params![undone],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn new_series_ids_stay_above_the_ids_an_undo_may_put_back() {
        let log = log();
        log.apply_pin_changes(&set(&[("x.jpg", manual(9))]))
            .unwrap();
        log.record_edit(
            EditKind::Ungroup,
            Path::new("x.jpg"),
            &set(&[("x.jpg", Pin::Single)]),
        )
        .unwrap()
        .unwrap();

        // No pin has id 9 now, but undoing the edit would bring it back.
        assert_eq!(log.next_series_id().unwrap(), ManualSeriesId(10));
    }
}
