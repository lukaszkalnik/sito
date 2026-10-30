//! Log of the actions Sito performs (file moves, accepting photos), in
//! `SQLite`, so every one of them can be undone (also after a restart).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

use crate::accept::{AcceptAction, AcceptChange};
use crate::reject::{FileMove, PhotoFiles};

/// Error while reading or writing the action log.
#[derive(Debug, Error)]
pub enum LogError {
    /// The database refused the operation.
    #[error("action log: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The path is not valid Unicode, so it cannot be stored without changing it.
    #[error("action log: {0} is not a valid Unicode path")]
    NonUnicodePath(PathBuf),

    /// A time in the log could not be read.
    #[error("action log: {0} is not a valid time")]
    BadTime(String),
}

/// An action in the log that has not been undone yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoggedAction {
    /// A photo moved into the rejected folder or the trash.
    Reject(RejectAction),
    /// A photo marked as accepted, or its mark taken off.
    Accept(AcceptAction),
}

/// A rejected photo that has not been undone yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectAction {
    /// Row id in the action log; identifies the action to undo.
    pub id: i64,
    /// Where the photo was before it was rejected.
    pub from: PathBuf,
    /// Where the photo went.
    pub place: Placement,
}

/// Where a rejected photo went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// Into the rejected folder next to it.
    Folder {
        /// Where the photo is now.
        to: PathBuf,
        /// The photo's other files, moved along with it.
        extra: Vec<FileMove>,
    },
    /// Into the system trash.
    Trash {
        /// The paths the files had, the photo first, then its other files.
        files: Vec<PathBuf>,
        /// When the move was logged, in Unix seconds (just before the move).
        at: i64,
    },
}

/// The action log, stored in one `SQLite` file.
#[derive(Debug)]
pub struct ActionLog {
    pub(crate) conn: Connection,
}

/// One entry of the `actions` table.
struct Row {
    id: i64,
    kind: String,
    at: String,
    from: PathBuf,
    to: PathBuf,
    /// A date file of entries written before migration 3.
    old_sidecar: Option<FileMove>,
}

const KIND_REJECT: &str = "reject";
const KIND_TRASH: &str = "trash";
pub(crate) const KIND_ACCEPT: &str = "accept";
pub(crate) const KIND_UNACCEPT: &str = "unaccept";

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
        self.record(KIND_REJECT, from, to, extra)
    }

    /// Records that the photo `files` are moved into the system trash;
    /// returns the action id. Like [`record_reject`](Self::record_reject),
    /// call it before moving and [`forget`](Self::forget) the entry if the
    /// move fails.
    ///
    /// # Errors
    ///
    /// Returns an error if a path is not valid Unicode or the entry cannot be
    /// written.
    pub fn record_trash(&self, files: &PhotoFiles) -> Result<i64, LogError> {
        // The trash has no path we choose, so each file is logged with its
        // own path on both sides.
        let others: Vec<FileMove> = files
            .all()
            .into_iter()
            .skip(1)
            .map(|path| FileMove {
                from: path.clone(),
                to: path,
            })
            .collect();
        self.record(KIND_TRASH, &files.photo, &files.photo, &others)
    }

    fn record(
        &self,
        kind: &str,
        from: &Path,
        to: &Path,
        extra: &[FileMove],
    ) -> Result<i64, LogError> {
        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO actions (at, kind, from_path, to_path) VALUES (?1, ?2, ?3, ?4)",
            params![now(), kind, path_text(from)?, path_text(to)?],
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

    /// The newest action not yet undone whose photo came from inside `under`
    /// (a folder and its subfolders), if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read.
    pub fn last_undoable(&self, under: &Path) -> Result<Option<LoggedAction>, LogError> {
        let Some(row) = self.last_row_under(under)? else {
            return Ok(None);
        };
        let accept = |change| {
            LoggedAction::Accept(AcceptAction {
                id: row.id,
                path: row.from.clone(),
                change,
            })
        };
        Ok(Some(match row.kind.as_str() {
            KIND_ACCEPT => accept(AcceptChange::Accepted),
            KIND_UNACCEPT => accept(AcceptChange::Unaccepted),
            _ => LoggedAction::Reject(self.reject_action(row)?),
        }))
    }

    /// The rejection logged in `row`, with the other files moved along.
    fn reject_action(&self, row: Row) -> Result<RejectAction, LogError> {
        let moves = self.moves_of(row.id)?;
        let place = if row.kind == KIND_TRASH {
            Placement::Trash {
                files: std::iter::once(row.from.clone())
                    .chain(moves.into_iter().map(|file| file.from))
                    .collect(),
                at: unix_seconds(&row.at)?,
            }
        } else {
            Placement::Folder {
                to: row.to,
                extra: row.old_sidecar.into_iter().chain(moves).collect(),
            }
        };
        Ok(RejectAction {
            id: row.id,
            from: row.from,
            place,
        })
    }

    /// The newest entry not yet undone whose photo came from inside `under`.
    fn last_row_under(&self, under: &Path) -> Result<Option<Row>, LogError> {
        let mut statement = self.conn.prepare(
            "SELECT id, kind, at, from_path, to_path, sidecar_from, sidecar_to FROM actions
             WHERE kind IN (?1, ?2, ?3, ?4) AND undone_at IS NULL
             ORDER BY id DESC",
        )?;
        let kinds = params![KIND_REJECT, KIND_TRASH, KIND_ACCEPT, KIND_UNACCEPT];
        let rows = statement.query_map(kinds, |row| {
            let sidecar_from: Option<String> = row.get(5)?;
            let sidecar_to: Option<String> = row.get(6)?;
            Ok(Row {
                id: row.get(0)?,
                kind: row.get(1)?,
                at: row.get(2)?,
                from: PathBuf::from(row.get::<_, String>(3)?),
                to: PathBuf::from(row.get::<_, String>(4)?),
                old_sidecar: sidecar_from.zip(sidecar_to).map(|(from, to)| FileMove {
                    from: from.into(),
                    to: to.into(),
                }),
            })
        })?;
        for row in rows {
            let row = row?;
            // Compares whole path components, so `Zdjęcia2` is not inside `Zdjęcia`.
            if row.from.starts_with(under) {
                return Ok(Some(row));
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

pub(crate) fn now() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string()
}

/// A time written by [`now`] (RFC 3339 with offset) as Unix seconds.
fn unix_seconds(at: &str) -> Result<i64, LogError> {
    DateTime::parse_from_rfc3339(at)
        .map(|time| time.timestamp())
        .map_err(|_| LogError::BadTime(at.to_owned()))
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
    // 4: photos marked as accepted, by path.
    "CREATE TABLE IF NOT EXISTS accepted (path TEXT PRIMARY KEY);",
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
pub(crate) fn path_text(path: &Path) -> Result<&str, LogError> {
    path.to_str()
        .ok_or_else(|| LogError::NonUnicodePath(path.to_path_buf()))
}
