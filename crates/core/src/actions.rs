//! Log of the file operations Sito performs, in `SQLite`, so every one of them
//! can be undone (also after a restart).

use std::path::{Path, PathBuf};

use chrono::Local;
use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

use crate::reject::FileMove;

/// Error while reading or writing the action log.
#[derive(Debug, Error)]
pub enum LogError {
    /// The database refused the operation.
    #[error("action log: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The path is not valid Unicode, so it cannot be stored without changing it.
    #[error("action log: {0} is not a valid Unicode path")]
    NonUnicodePath(PathBuf),
}

/// A photo moved into the rejected folder that has not been undone yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectAction {
    /// Row id in the action log; identifies the action to undo.
    pub id: i64,
    /// Where the photo was before it was rejected.
    pub from: PathBuf,
    /// Where the photo is now.
    pub to: PathBuf,
    /// The photo's other files, moved along with it.
    pub extra: Vec<FileMove>,
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

    fn init(mut conn: Connection) -> Result<Self, LogError> {
        migrate(&mut conn)?;
        Ok(Self { conn })
    }

    /// Records that the photo at `from` is moved to `to`, with its other
    /// files `extra`; returns the action id.
    /// Call it before moving, and [`forget`](Self::forget) the entry if the
    /// move then fails, so no move is ever left out of the log.
    ///
    /// # Errors
    ///
    /// Returns an error if a path is not valid Unicode or the entry cannot be
    /// written.
    pub fn record_reject(
        &self,
        from: &Path,
        to: &Path,
        extra: &[FileMove],
    ) -> Result<i64, LogError> {
        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO actions (at, kind, from_path, to_path) VALUES (?1, ?2, ?3, ?4)",
            params![now(), KIND_REJECT, path_text(from)?, path_text(to)?],
        )?;
        let id = transaction.last_insert_rowid();
        for file in extra {
            transaction.execute(
                "INSERT INTO action_moves (action_id, from_path, to_path) VALUES (?1, ?2, ?3)",
                params![id, path_text(&file.from)?, path_text(&file.to)?],
            )?;
        }
        transaction.commit()?;
        Ok(id)
    }

    /// Removes the entry `id` of an action that did not happen after all.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry cannot be removed.
    pub fn forget(&self, id: i64) -> Result<(), LogError> {
        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute("DELETE FROM action_moves WHERE action_id = ?1", params![id])?;
        transaction.execute("DELETE FROM actions WHERE id = ?1", params![id])?;
        transaction.commit()?;
        Ok(())
    }

    /// The newest rejection not yet undone whose photo came from inside
    /// `under` (a folder and its subfolders), if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read.
    pub fn last_undoable(&self, under: &Path) -> Result<Option<RejectAction>, LogError> {
        let mut statement = self.conn.prepare(
            "SELECT id, from_path, to_path, sidecar_from, sidecar_to FROM actions
             WHERE kind = ?1 AND undone_at IS NULL
             ORDER BY id DESC",
        )?;
        let rows = statement.query_map(params![KIND_REJECT], |row| {
            let sidecar_from: Option<String> = row.get(3)?;
            let sidecar_to: Option<String> = row.get(4)?;
            Ok(RejectAction {
                id: row.get(0)?,
                from: PathBuf::from(row.get::<_, String>(1)?),
                to: PathBuf::from(row.get::<_, String>(2)?),
                // Entries written before migration 3 keep a date file here.
                extra: sidecar_from
                    .zip(sidecar_to)
                    .map(|(from, to)| FileMove {
                        from: from.into(),
                        to: to.into(),
                    })
                    .into_iter()
                    .collect(),
            })
        })?;
        for row in rows {
            let mut action = row?;
            // Compares whole path components, so `Zdjęcia2` is not inside `Zdjęcia`.
            if action.from.starts_with(under) {
                action.extra.extend(self.moves_of(action.id)?);
                return Ok(Some(action));
            }
        }
        Ok(None)
    }

    /// The other files moved with the action `id`, in the order they moved.
    fn moves_of(&self, id: i64) -> Result<Vec<FileMove>, LogError> {
        let mut statement = self.conn.prepare(
            "SELECT from_path, to_path FROM action_moves WHERE action_id = ?1 ORDER BY rowid",
        )?;
        let rows = statement.query_map(params![id], |row| {
            Ok(FileMove {
                from: PathBuf::from(row.get::<_, String>(0)?),
                to: PathBuf::from(row.get::<_, String>(1)?),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
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

/// Schema changes of the log, applied in order. `PRAGMA user_version` holds
/// how many have been applied. Never edit or reorder an entry; append a new one.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS actions (
         id        INTEGER PRIMARY KEY,
         at        TEXT NOT NULL,
         kind      TEXT NOT NULL,
         from_path TEXT NOT NULL,
         to_path   TEXT NOT NULL,
         undone_at TEXT
     );",
    // 2: a photo's Google Takeout date file moved along with it (replaced by 3,
    // still read for older entries).
    "ALTER TABLE actions ADD COLUMN sidecar_from TEXT;
     ALTER TABLE actions ADD COLUMN sidecar_to TEXT;",
    // 3: any number of files moved along with a photo.
    "CREATE TABLE IF NOT EXISTS action_moves (
         action_id INTEGER NOT NULL REFERENCES actions(id),
         from_path TEXT NOT NULL,
         to_path   TEXT NOT NULL
     );",
];

/// Applies the [`MIGRATIONS`] the database at `conn` does not have yet, in one
/// transaction.
fn migrate(conn: &mut Connection) -> Result<(), LogError> {
    let applied: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let transaction = conn.transaction()?;
    let mut version = applied;
    for migration in MIGRATIONS.iter().skip(applied as usize) {
        transaction.execute_batch(migration)?;
        version += 1;
    }
    transaction.pragma_update(None, "user_version", version)?;
    transaction.commit()?;
    Ok(())
}

/// The path as text. Paths that are not valid Unicode (practically never seen
/// on Windows or macOS) are refused instead of being stored changed.
fn path_text(path: &Path) -> Result<&str, LogError> {
    path.to_str()
        .ok_or_else(|| LogError::NonUnicodePath(path.to_path_buf()))
}
