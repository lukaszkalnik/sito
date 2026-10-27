//! Cache of what scans read from photos (capture time, orientation, camera),
//! in SQLite, so rescanning a folder only reads new or changed files and a
//! folder can be shown at once from the cache before it is rescanned.
//!
//! The cache can always be rebuilt from the photos, so it lives apart from
//! the action log, and anything unreadable is simply dropped.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::NaiveDateTime;
use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

use crate::metadata::Orientation;
use crate::timeline::{Photo, TimeSource};

/// Changes whenever what a scan reads from a photo changes; a cache written
/// by another version is emptied, so every photo is read again once.
pub const CACHE_VERSION: i64 = 1;

const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f";

/// Error while reading or writing the photo cache.
#[derive(Debug, Error)]
#[error("photo cache: {0}")]
pub struct CacheError(#[from] rusqlite::Error);

/// The photo cache, stored in one SQLite file.
#[derive(Debug)]
pub struct PhotoCache {
    conn: Connection,
}

impl PhotoCache {
    /// Opens (or creates) the cache in the file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or is not a valid cache;
    /// the caller may then delete it and try again.
    pub fn open(path: &Path) -> Result<Self, CacheError> {
        Self::init(Connection::open(path)?)
    }

    /// A cache kept in memory only, for tests.
    ///
    /// # Errors
    ///
    /// Returns an error if SQLite cannot set up the tables.
    pub fn open_in_memory() -> Result<Self, CacheError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, CacheError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS photos (
                 path        TEXT PRIMARY KEY,
                 size        INTEGER NOT NULL,
                 mtime_ns    INTEGER,
                 taken_at    TEXT NOT NULL,
                 exif        INTEGER NOT NULL,
                 orientation INTEGER NOT NULL,
                 camera      TEXT
             );",
        )?;
        let version: Option<i64> = conn
            .query_row("SELECT value FROM meta WHERE key = 'version'", [], |row| {
                row.get(0)
            })
            .optional()?;
        if version != Some(CACHE_VERSION) {
            conn.execute("DELETE FROM photos", [])?;
            conn.execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('version', ?1)",
                params![CACHE_VERSION],
            )?;
        }
        Ok(Self { conn })
    }

    /// The cached photos inside `root` (and its subfolders), in no particular order.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache cannot be read.
    pub fn photos_under(&self, root: &Path) -> Result<Vec<Photo>, CacheError> {
        Ok(self
            .all()?
            .into_iter()
            .filter(|photo| photo.path.starts_with(root))
            .collect())
    }

    /// All cached photos, of every folder ever scanned.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache cannot be read.
    pub fn all(&self) -> Result<Vec<Photo>, CacheError> {
        let mut statement = self.conn.prepare(
            "SELECT path, size, mtime_ns, taken_at, exif, orientation, camera FROM photos",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, bool>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })?;
        let mut photos = Vec::new();
        for row in rows {
            let (path, size, mtime_ns, taken_at, exif, orientation, camera) = row?;
            // A row that does not parse is skipped; the photo is read again.
            let Ok(taken_at) = NaiveDateTime::parse_from_str(&taken_at, TIME_FORMAT) else {
                continue;
            };
            photos.push(Photo {
                path: PathBuf::from(path),
                size: u64::try_from(size).unwrap_or_default(),
                modified: mtime_ns.and_then(from_nanos),
                taken_at,
                time_source: if exif {
                    TimeSource::Exif
                } else {
                    TimeSource::FileModified
                },
                orientation: Orientation::from_exif(u16::try_from(orientation).unwrap_or(1)),
                camera,
            });
        }
        Ok(photos)
    }

    /// Stores `photos`, replacing earlier entries for the same paths.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache cannot be written; then nothing is stored.
    pub fn store(&mut self, photos: &[Photo]) -> Result<(), CacheError> {
        let transaction = self.conn.transaction()?;
        {
            let mut insert = transaction.prepare(
                "INSERT OR REPLACE INTO photos
                     (path, size, mtime_ns, taken_at, exif, orientation, camera)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for photo in photos {
                insert.execute(params![
                    photo.path.to_string_lossy(),
                    i64::try_from(photo.size).unwrap_or(i64::MAX),
                    photo.modified.and_then(to_nanos),
                    photo.taken_at.format(TIME_FORMAT).to_string(),
                    photo.time_source == TimeSource::Exif,
                    i64::from(photo.orientation.value()),
                    photo.camera,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Removes the cached photos inside `root` that are not in `found`: a
    /// scan of `root` no longer found them (deleted, moved or renamed).
    ///
    /// # Errors
    ///
    /// Returns an error if the cache cannot be written.
    pub fn forget_missing(
        &mut self,
        root: &Path,
        found: &HashSet<PathBuf>,
    ) -> Result<(), CacheError> {
        let gone: Vec<PathBuf> = self
            .photos_under(root)?
            .into_iter()
            .map(|photo| photo.path)
            .filter(|path| !found.contains(path))
            .collect();
        let transaction = self.conn.transaction()?;
        for path in &gone {
            transaction.execute(
                "DELETE FROM photos WHERE path = ?1",
                params![path.to_string_lossy()],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }
}

fn to_nanos(time: SystemTime) -> Option<i64> {
    let since = time.duration_since(UNIX_EPOCH).ok()?;
    i64::try_from(since.as_nanos()).ok()
}

fn from_nanos(nanos: i64) -> Option<SystemTime> {
    let nanos = u64::try_from(nanos).ok()?;
    UNIX_EPOCH.checked_add(Duration::from_nanos(nanos))
}
