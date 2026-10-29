//! Cache of what scans read from photos (capture time, orientation, camera),
//! in `SQLite`, so rescanning a folder only reads new or changed files and a
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
use crate::scan::{FileStamp, StampedFile};
use crate::takeout::TakeoutUse;
use crate::timeline::{ExifInfo, Photo, TimeSource};

/// Changes whenever what a scan reads from a photo changes; a cache written
/// by another version is emptied, so every photo is read again once.
pub const CACHE_VERSION: i64 = 4;

const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f";

/// Error while reading or writing the photo cache.
#[derive(Debug, Error)]
#[error("photo cache: {0}")]
pub struct CacheError(#[from] rusqlite::Error);

/// The photo cache, stored in one `SQLite` file.
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
    /// Returns an error if `SQLite` cannot set up the tables.
    pub fn open_in_memory() -> Result<Self, CacheError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, CacheError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL)",
            [],
        )?;
        let version: Option<i64> = conn
            .query_row("SELECT value FROM meta WHERE key = 'version'", [], |row| {
                row.get(0)
            })
            .optional()?;
        if version != Some(CACHE_VERSION) {
            // A cache of another version may have other columns: start over.
            conn.execute("DROP TABLE IF EXISTS photos", [])?;
            conn.execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('version', ?1)",
                params![CACHE_VERSION],
            )?;
        }
        conn.execute(
            "CREATE TABLE IF NOT EXISTS photos (
                 path             TEXT PRIMARY KEY,
                 size             INTEGER NOT NULL,
                 mtime_ns         INTEGER,
                 taken_at         TEXT NOT NULL,
                 time_source      TEXT NOT NULL,
                 orientation      INTEGER,
                 camera           TEXT,
                 sidecar_path     TEXT,
                 sidecar_size     INTEGER,
                 sidecar_mtime_ns INTEGER,
                 companion_path     TEXT,
                 companion_size     INTEGER,
                 companion_mtime_ns INTEGER
             )",
            [],
        )?;
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

    /// The cached photos inside `root` whose EXIF is still pending, in path
    /// order (the order a scan finds them in).
    ///
    /// # Errors
    ///
    /// Returns an error if the cache cannot be read.
    pub fn pending_under(&self, root: &Path) -> Result<Vec<Photo>, CacheError> {
        let mut pending: Vec<Photo> = self
            .photos_under(root)?
            .into_iter()
            .filter(|photo| photo.exif == ExifInfo::Pending)
            .collect();
        pending.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(pending)
    }

    /// All cached photos, of every folder ever scanned.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache cannot be read.
    pub fn all(&self) -> Result<Vec<Photo>, CacheError> {
        let mut statement = self.conn.prepare(
            "SELECT path, size, mtime_ns, taken_at, time_source, orientation, camera,
                    sidecar_path, sidecar_size, sidecar_mtime_ns,
                    companion_path, companion_size, companion_mtime_ns
             FROM photos",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Row {
                path: row.get(0)?,
                size: row.get(1)?,
                mtime_ns: row.get(2)?,
                taken_at: row.get(3)?,
                time_source: row.get(4)?,
                orientation: row.get(5)?,
                camera: row.get(6)?,
                sidecar: (row.get(7)?, row.get(8)?, row.get(9)?),
                companion: (row.get(10)?, row.get(11)?, row.get(12)?),
            })
        })?;
        let mut photos = Vec::new();
        for row in rows {
            // A row that does not parse is skipped; the photo is read again.
            if let Some(photo) = row?.into_photo() {
                photos.push(photo);
            }
        }
        Ok(photos)
    }

    /// Stores `photos`, replacing earlier entries for the same paths. Photos
    /// whose path is not valid Unicode are left out (they are read on every
    /// scan) rather than stored under a changed path.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache cannot be written; then nothing is stored.
    pub fn store(&mut self, photos: &[Photo]) -> Result<(), CacheError> {
        let transaction = self.conn.transaction()?;
        {
            let mut insert = transaction.prepare(
                "INSERT OR REPLACE INTO photos
                     (path, size, mtime_ns, taken_at, time_source, orientation, camera,
                      sidecar_path, sidecar_size, sidecar_mtime_ns,
                      companion_path, companion_size, companion_mtime_ns)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )?;
            for photo in photos {
                // Paths that are not valid Unicode are not stored (see above).
                let (Some(path), Some(sidecar), Some(companion)) = (
                    photo.path.to_str(),
                    stamp_columns(photo.sidecar.as_ref()),
                    stamp_columns(photo.companion.as_ref()),
                ) else {
                    continue;
                };
                let (orientation, camera) = match &photo.exif {
                    ExifInfo::Read {
                        orientation,
                        camera,
                    } => (Some(i64::from(orientation.value())), camera.as_deref()),
                    ExifInfo::Pending => (None, None),
                };
                insert.execute(params![
                    path,
                    i64::try_from(photo.size).unwrap_or(i64::MAX),
                    photo.modified.and_then(to_nanos),
                    photo.taken_at.format(TIME_FORMAT).to_string(),
                    source_code(photo.time_source),
                    orientation,
                    camera,
                    sidecar.0,
                    sidecar.1,
                    sidecar.2,
                    companion.0,
                    companion.1,
                    companion.2,
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
        // Cached paths are always valid Unicode (see `store`).
        for path in gone.iter().filter_map(|path| path.to_str()) {
            transaction.execute("DELETE FROM photos WHERE path = ?1", params![path])?;
        }
        transaction.commit()?;
        Ok(())
    }
}

/// A stored file reference: path, size and modification time.
type StampColumns = (Option<String>, Option<i64>, Option<i64>);

/// One row of the `photos` table.
struct Row {
    path: String,
    size: i64,
    mtime_ns: Option<i64>,
    taken_at: String,
    time_source: String,
    /// `NULL` while the photo's EXIF is pending.
    orientation: Option<i64>,
    camera: Option<String>,
    sidecar: StampColumns,
    companion: StampColumns,
}

impl Row {
    fn into_photo(self) -> Option<Photo> {
        let taken_at = NaiveDateTime::parse_from_str(&self.taken_at, TIME_FORMAT).ok()?;
        let exif = match self.orientation {
            Some(value) => ExifInfo::Read {
                orientation: Orientation::from_exif(u16::try_from(value).unwrap_or(1)),
                camera: self.camera,
            },
            None => ExifInfo::Pending,
        };
        Some(Photo {
            path: PathBuf::from(self.path),
            size: u64::try_from(self.size).unwrap_or_default(),
            modified: self.mtime_ns.and_then(from_nanos),
            taken_at,
            time_source: source_from_code(&self.time_source)?,
            exif,
            sidecar: stamped_file(self.sidecar),
            companion: stamped_file(self.companion),
        })
    }
}

/// The columns of an optional file reference; `None` if its path is not
/// valid Unicode and cannot be stored unchanged.
fn stamp_columns(file: Option<&StampedFile>) -> Option<(Option<&str>, Option<i64>, Option<i64>)> {
    let Some(file) = file else {
        return Some((None, None, None));
    };
    Some((
        Some(file.path.to_str()?),
        Some(i64::try_from(file.stamp.size).unwrap_or(i64::MAX)),
        file.stamp.modified.and_then(to_nanos),
    ))
}

fn stamped_file((path, size, mtime_ns): StampColumns) -> Option<StampedFile> {
    Some(StampedFile {
        path: PathBuf::from(path?),
        stamp: FileStamp {
            size: size
                .and_then(|size| u64::try_from(size).ok())
                .unwrap_or_default(),
            modified: mtime_ns.and_then(from_nanos),
        },
    })
}

/// How a [`TimeSource`] is stored.
fn source_code(source: TimeSource) -> &'static str {
    match source {
        TimeSource::Exif => "exif",
        TimeSource::Takeout(TakeoutUse::MissingExif) => "takeout",
        TimeSource::Takeout(TakeoutUse::ImplausibleExif) => "takeout-implausible-exif",
        TimeSource::Takeout(TakeoutUse::Corrected) => "takeout-corrected",
        TimeSource::FileModified => "file",
    }
}

fn source_from_code(code: &str) -> Option<TimeSource> {
    [
        TimeSource::Exif,
        TimeSource::Takeout(TakeoutUse::MissingExif),
        TimeSource::Takeout(TakeoutUse::ImplausibleExif),
        TimeSource::Takeout(TakeoutUse::Corrected),
        TimeSource::FileModified,
    ]
    .into_iter()
    .find(|&source| source_code(source) == code)
}

fn to_nanos(time: SystemTime) -> Option<i64> {
    let since = time.duration_since(UNIX_EPOCH).ok()?;
    i64::try_from(since.as_nanos()).ok()
}

fn from_nanos(nanos: i64) -> Option<SystemTime> {
    let nanos = u64::try_from(nanos).ok()?;
    UNIX_EPOCH.checked_add(Duration::from_nanos(nanos))
}
