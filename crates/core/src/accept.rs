//! Photos the user marked as accepted (the keepers of a series), stored by
//! path in the action log's database so the mark survives restarts and
//! rescans, and every change of it can be undone.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::{Transaction, params};

use crate::actions::{ActionLog, KIND_ACCEPT, KIND_UNACCEPT, LogError, now, path_text};

/// How an accept action changed a photo's mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AcceptChange {
    /// The photo was marked as accepted.
    Accepted,
    /// The photo's mark was taken off.
    Unaccepted,
}

impl AcceptChange {
    /// The change that reverts this one.
    #[must_use]
    pub fn reverse(self) -> Self {
        match self {
            Self::Accepted => Self::Unaccepted,
            Self::Unaccepted => Self::Accepted,
        }
    }

    fn kind(self) -> &'static str {
        match self {
            Self::Accepted => KIND_ACCEPT,
            Self::Unaccepted => KIND_UNACCEPT,
        }
    }
}

/// A change of a photo's accepted mark that has not been undone yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptAction {
    /// Row id in the action log; identifies the action to undo.
    pub id: i64,
    /// The photo whose mark changed.
    pub path: PathBuf,
    /// How the mark changed.
    pub change: AcceptChange,
}

impl ActionLog {
    /// Marks the photo at `path` as accepted or takes the mark off, and logs
    /// it, in one transaction; returns the action id. If the photo already
    /// is in that state nothing changes or is logged, and it returns `None`.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is not valid Unicode or the database
    /// cannot be written.
    pub fn set_accepted(&self, path: &Path, change: AcceptChange) -> Result<Option<i64>, LogError> {
        let transaction = self.conn.unchecked_transaction()?;
        if write_mark(&transaction, path, change)? == 0 {
            return Ok(None);
        }
        let text = path_text(path)?;
        transaction.execute(
            "INSERT INTO actions (at, kind, from_path, to_path) VALUES (?1, ?2, ?3, ?3)",
            params![now(), change.kind(), text],
        )?;
        let id = transaction.last_insert_rowid();
        transaction.commit()?;
        Ok(Some(id))
    }

    /// Reverts `action` (an accept takes the mark off, an un-accept puts it
    /// back) and marks it undone, in one transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is not valid Unicode or the database
    /// cannot be written.
    pub fn undo_accept(&self, action: &AcceptAction) -> Result<(), LogError> {
        let transaction = self.conn.unchecked_transaction()?;
        write_mark(&transaction, &action.path, action.change.reverse())?;
        transaction.execute(
            "UPDATE actions SET undone_at = ?1 WHERE id = ?2",
            params![now(), action.id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// The accepted photos inside `under` (a folder and its subfolders).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn accepted_paths(&self, under: &Path) -> Result<HashSet<PathBuf>, LogError> {
        let mut statement = self.conn.prepare("SELECT path FROM accepted")?;
        let paths = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut inside = HashSet::new();
        for path in paths {
            let path = PathBuf::from(path?);
            // Compares whole path components, so `Zdjęcia2` is not inside `Zdjęcia`.
            if path.starts_with(under) {
                inside.insert(path);
            }
        }
        Ok(inside)
    }
}

/// Adds or removes the mark of `path`; returns how many rows changed (0 if
/// it already was in that state).
fn write_mark(
    transaction: &Transaction<'_>,
    path: &Path,
    change: AcceptChange,
) -> Result<usize, LogError> {
    let sql = match change {
        AcceptChange::Accepted => "INSERT OR IGNORE INTO accepted (path) VALUES (?1)",
        AcceptChange::Unaccepted => "DELETE FROM accepted WHERE path = ?1",
    };
    Ok(transaction.execute(sql, params![path_text(path)?])?)
}
