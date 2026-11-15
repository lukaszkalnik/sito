//! Photos the user marked as accepted (the keepers of a series), stored by
//! path in the action log's database so the mark survives restarts and
//! rescans. Changing a mark is not logged as an action: the user takes a mark
//! back by toggling it, and undo is only for rejections.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::params;

use crate::actions::{ActionLog, LogError, path_text};
use crate::moved::MovedPhoto;

/// How to change a photo's accepted mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AcceptChange {
    /// Mark the photo as accepted.
    Accepted,
    /// Take the photo's mark off.
    Unaccepted,
}

impl ActionLog {
    /// Marks the photo at `path` as accepted or takes the mark off. If the
    /// photo already is in that state nothing changes.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is not valid Unicode or the database
    /// cannot be written.
    pub fn set_accepted(&self, path: &Path, change: AcceptChange) -> Result<(), LogError> {
        let sql = match change {
            AcceptChange::Accepted => "INSERT OR IGNORE INTO accepted (path) VALUES (?1)",
            AcceptChange::Unaccepted => "DELETE FROM accepted WHERE path = ?1",
        };
        self.conn.execute(sql, params![path_text(path)?])?;
        Ok(())
    }

    /// Moves the accepted marks of photos moved outside Sito to their new
    /// paths, all in one transaction. Photos without a mark are left as
    /// they are; a mark already at a new path stays.
    ///
    /// # Errors
    ///
    /// Returns an error if a path is not valid Unicode or the database
    /// cannot be written; then no mark is moved.
    pub fn move_accepted(&self, moves: &[MovedPhoto]) -> Result<(), LogError> {
        let transaction = self.conn.unchecked_transaction()?;
        for moved in moves {
            let from = path_text(&moved.from)?;
            let removed =
                transaction.execute("DELETE FROM accepted WHERE path = ?1", params![from])?;
            if removed > 0 {
                transaction.execute(
                    "INSERT OR IGNORE INTO accepted (path) VALUES (?1)",
                    params![path_text(&moved.to)?],
                )?;
            }
        }
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
