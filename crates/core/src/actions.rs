//! Log of the file operations Sito performs, in `SQLite`, so every one of them
//! can be undone (also after a restart).

use std::path::{Path, PathBuf};

use chrono::Local;
use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

/// Error while reading or writing the action log.
#[derive(Debug, Error)]
#[error("action log: {0}")]
pub struct LogError(#[from] rusqlite::Error);

/// A photo moved into the rejected folder that has not been undone yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectAction {
    /// Row id in the action log; identifies the action to undo.
    pub id: i64,
    /// Where the photo was before it was rejected.
    pub from: PathBuf,
    /// Where the photo is now.
    pub to: PathBuf,
}

/// The action log, stored in one `SQLite` file.
#[derive(Debug)]
pub struct ActionLog {
    conn: Connection,
}

const KIND_REJECT: &str = "reject";

impl ActionLog {
    /// Opens (or creates) the log in the file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or is not a valid log.
    pub fn open(path: &Path) -> Result<Self, LogError> {
        Self::init(Connection::open(path)?)
    }

    /// A log kept in memory only, for tests.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot set up the table.
    pub fn open_in_memory() -> Result<Self, LogError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, LogError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS actions (
                 id        INTEGER PRIMARY KEY,
                 at        TEXT NOT NULL,
                 kind      TEXT NOT NULL,
                 from_path TEXT NOT NULL,
                 to_path   TEXT NOT NULL,
                 undone_at TEXT
             );",
        )?;
        Ok(Self { conn })
    }

    /// Records that the photo at `from` was moved to `to`; returns the action id.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry cannot be written.
    pub fn record_reject(&self, from: &Path, to: &Path) -> Result<i64, LogError> {
        self.conn.execute(
            "INSERT INTO actions (at, kind, from_path, to_path) VALUES (?1, ?2, ?3, ?4)",
            params![now(), KIND_REJECT, path_text(from), path_text(to)],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// The newest rejection not yet undone whose photo came from inside
    /// `under` (a folder and its subfolders), if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read.
    pub fn last_undoable(&self, under: &Path) -> Result<Option<RejectAction>, LogError> {
        let mut statement = self.conn.prepare(
            "SELECT id, from_path, to_path FROM actions
             WHERE kind = ?1 AND undone_at IS NULL
             ORDER BY id DESC",
        )?;
        let rows = statement.query_map(params![KIND_REJECT], |row| {
            Ok(RejectAction {
                id: row.get(0)?,
                from: PathBuf::from(row.get::<_, String>(1)?),
                to: PathBuf::from(row.get::<_, String>(2)?),
            })
        })?;
        for row in rows {
            let action = row?;
            // Compares whole path components, so `Zdjęcia2` is not inside `Zdjęcia`.
            if action.from.starts_with(under) {
                return Ok(Some(action));
            }
        }
        Ok(None)
    }

    /// Marks the action `id` as undone, so it is not offered again.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry cannot be written.
    pub fn mark_undone(&self, id: i64) -> Result<(), LogError> {
        self.conn.execute(
            "UPDATE actions SET undone_at = ?1 WHERE id = ?2",
            params![now(), id],
        )?;
        Ok(())
    }

    /// When the action `id` was undone, if it was.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read.
    pub fn undone_at(&self, id: i64) -> Result<Option<String>, LogError> {
        Ok(self
            .conn
            .query_row(
                "SELECT undone_at FROM actions WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?
            .flatten())
    }
}

fn now() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string()
}

/// Paths are stored as text; Windows file names that are not valid Unicode
/// (practically never seen) would be stored lossily.
fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
