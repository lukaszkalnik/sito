//! Pins of edited series in the action log's database, by path, so edits
//! survive restarts and rescans. Edits are not logged as actions: the user
//! takes one back with „Przywróć automatyczne grupowanie”, and undo is only for
//! rejections.

use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use super::pins::{ManualSeriesId, Pin, PinChanges, Pins};
use crate::actions::{ActionLog, LogError, path_text};
use crate::moved::MovedPhoto;

impl ActionLog {
    /// The pins of the photos inside `under` (a folder and its subfolders).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn pins_under(&self, under: &Path) -> Result<Pins, LogError> {
        let mut statement = self.conn.prepare("SELECT path, series FROM series_pins")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
        })?;
        let mut inside = Vec::new();
        for row in rows {
            let (path, series) = row?;
            let path = PathBuf::from(path);
            // Compares whole path components, so `Zdjęcia2` is not inside `Zdjęcia`.
            if path.starts_with(under) {
                inside.push((path, pin_of(series)));
            }
        }
        Ok(inside.into_iter().collect())
    }

    /// An id no manual series has yet, in any folder.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn next_series_id(&self) -> Result<ManualSeriesId, LogError> {
        let highest: Option<i64> =
            self.conn
                .query_row("SELECT MAX(series) FROM series_pins", [], |row| row.get(0))?;
        Ok(ManualSeriesId(highest.map_or(1, |id| id.saturating_add(1))))
    }

    /// Applies `changes` (first the pins to clear, then those to set) in one
    /// transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if a path is not valid Unicode or the database
    /// cannot be written; then no pin changes.
    pub fn apply_pin_changes(&self, changes: &PinChanges) -> Result<(), LogError> {
        let transaction = self.conn.unchecked_transaction()?;
        for path in &changes.clear {
            transaction.execute(
                "DELETE FROM series_pins WHERE path = ?1",
                params![path_text(path)?],
            )?;
        }
        for (path, pin) in &changes.set {
            transaction.execute(
                "INSERT OR REPLACE INTO series_pins (path, series) VALUES (?1, ?2)",
                params![path_text(path)?, series_of(*pin)],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Moves the pins of photos moved outside Sito to their new paths, all
    /// in one transaction. Photos without a pin are left as they are; a pin
    /// already at a new path stays.
    ///
    /// # Errors
    ///
    /// Returns an error if a path is not valid Unicode or the database
    /// cannot be written; then no pin is moved.
    pub fn move_pins(&self, moves: &[MovedPhoto]) -> Result<(), LogError> {
        let transaction = self.conn.unchecked_transaction()?;
        for moved in moves {
            let series: Option<Option<i64>> = transaction
                .query_row(
                    "DELETE FROM series_pins WHERE path = ?1 RETURNING series",
                    params![path_text(&moved.from)?],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(series) = series {
                transaction.execute(
                    "INSERT OR IGNORE INTO series_pins (path, series) VALUES (?1, ?2)",
                    params![path_text(&moved.to)?, series],
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }
}

/// The pin stored as `series` (NULL for a single photo).
fn pin_of(series: Option<i64>) -> Pin {
    series.map_or(Pin::Single, |id| Pin::Series(ManualSeriesId(id)))
}

/// How `pin` is stored.
fn series_of(pin: Pin) -> Option<i64> {
    match pin {
        Pin::Series(ManualSeriesId(id)) => Some(id),
        Pin::Single => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log() -> ActionLog {
        ActionLog::open_in_memory().unwrap()
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

    const ONE: Pin = Pin::Series(ManualSeriesId(1));

    #[test]
    fn stores_pins_and_reads_those_of_a_folder() {
        let log = log();

        log.apply_pin_changes(&set(&[
            ("a/x.jpg", ONE),
            ("a/sub/y.jpg", Pin::Single),
            ("a2/z.jpg", ONE),
        ]))
        .unwrap();

        let pins = log.pins_under(Path::new("a")).unwrap();
        assert_eq!(pins.get(Path::new("a/x.jpg")), Some(ONE));
        assert_eq!(pins.get(Path::new("a/sub/y.jpg")), Some(Pin::Single));
        assert_eq!(pins.get(Path::new("a2/z.jpg")), None);
    }

    #[test]
    fn changes_replace_and_clear_pins() {
        let log = log();
        log.apply_pin_changes(&set(&[("x.jpg", ONE), ("y.jpg", ONE)]))
            .unwrap();

        log.apply_pin_changes(&PinChanges {
            set: vec![(PathBuf::from("x.jpg"), Pin::Single)],
            clear: vec![PathBuf::from("y.jpg")],
        })
        .unwrap();

        let pins = log.pins_under(Path::new("")).unwrap();
        assert_eq!(pins.get(Path::new("x.jpg")), Some(Pin::Single));
        assert_eq!(pins.get(Path::new("y.jpg")), None);
    }

    #[test]
    fn new_series_ids_follow_the_highest_of_any_folder() {
        let log = log();
        assert_eq!(log.next_series_id().unwrap(), ManualSeriesId(1));

        log.apply_pin_changes(&set(&[
            ("b/x.jpg", Pin::Series(ManualSeriesId(4))),
            ("a/y.jpg", Pin::Single),
        ]))
        .unwrap();

        assert_eq!(log.next_series_id().unwrap(), ManualSeriesId(5));
    }

    #[test]
    fn pins_move_with_moved_photos() {
        let log = log();
        log.apply_pin_changes(&set(&[("a/x.jpg", ONE), ("a/taken.jpg", Pin::Single)]))
            .unwrap();
        let moved = |from: &str, to: &str| MovedPhoto {
            from: PathBuf::from(from),
            to: PathBuf::from(to),
        };

        log.move_pins(&[
            moved("a/x.jpg", "b/x.jpg"),
            moved("a/none.jpg", "b/none.jpg"),
            moved("a/taken.jpg", "a/x2.jpg"),
        ])
        .unwrap();

        let pins = log.pins_under(Path::new("")).unwrap();
        assert_eq!(pins.get(Path::new("a/x.jpg")), None);
        assert_eq!(pins.get(Path::new("b/x.jpg")), Some(ONE));
        assert_eq!(pins.get(Path::new("b/none.jpg")), None);
        assert_eq!(pins.get(Path::new("a/x2.jpg")), Some(Pin::Single));
    }

    #[test]
    fn a_pin_already_at_the_new_path_stays() {
        let log = log();
        log.apply_pin_changes(&set(&[("x.jpg", ONE), ("y.jpg", Pin::Single)]))
            .unwrap();

        log.move_pins(&[MovedPhoto {
            from: PathBuf::from("x.jpg"),
            to: PathBuf::from("y.jpg"),
        }])
        .unwrap();

        let pins = log.pins_under(Path::new("")).unwrap();
        assert_eq!(pins.get(Path::new("y.jpg")), Some(Pin::Single));
        assert_eq!(pins.get(Path::new("x.jpg")), None);
    }
}
