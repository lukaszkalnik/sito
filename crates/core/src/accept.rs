//! Photos the user marked as accepted (the keepers of a series), stored by
//! path in the action log's database so the mark survives restarts and
//! rescans. Changing a mark is not logged as an action: the user takes a mark
//! back by toggling it, and undo is only for rejections.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::params;

use crate::actions::{ActionLog, LogError, path_text};

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
