//! The user's settings, as text values by key, stored in the action log's
//! database so they survive restarts and live outside the caches (which can
//! be emptied). What a key means and which values it takes is up to the
//! caller; this module only stores them.

use rusqlite::{OptionalExtension, params};

use crate::actions::{ActionLog, LogError};

impl ActionLog {
    /// The value of the setting `key`, or `None` if it was never set.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn setting(&self, key: &str) -> Result<Option<String>, LogError> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Sets the setting `key` to `value`, replacing a value it had.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), LogError> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log() -> ActionLog {
        ActionLog::open_in_memory().unwrap()
    }

    #[test]
    fn a_setting_never_set_is_none() {
        assert_eq!(log().setting("language").unwrap(), None);
    }

    #[test]
    fn a_set_setting_reads_back() {
        let log = log();
        log.set_setting("language", "de").unwrap();
        assert_eq!(log.setting("language").unwrap().as_deref(), Some("de"));
    }

    #[test]
    fn setting_again_replaces_the_value() {
        let log = log();
        log.set_setting("language", "de").unwrap();
        log.set_setting("language", "en").unwrap();
        assert_eq!(log.setting("language").unwrap().as_deref(), Some("en"));
    }

    #[test]
    fn keys_are_separate() {
        let log = log();
        log.set_setting("language", "de").unwrap();
        log.set_setting("other", "x").unwrap();
        assert_eq!(log.setting("language").unwrap().as_deref(), Some("de"));
        assert_eq!(log.setting("other").unwrap().as_deref(), Some("x"));
    }

    #[test]
    fn values_keep_non_ascii_text() {
        let log = log();
        log.set_setting("folder", "Zdjęcia – ąćęłńóśźż").unwrap();
        assert_eq!(
            log.setting("folder").unwrap().as_deref(),
            Some("Zdjęcia – ąćęłńóśźż")
        );
    }

    #[test]
    fn settings_survive_reopening_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sito.db");
        ActionLog::open(&path)
            .unwrap()
            .set_setting("language", "pl")
            .unwrap();
        let reopened = ActionLog::open(&path).unwrap();
        assert_eq!(reopened.setting("language").unwrap().as_deref(), Some("pl"));
    }

    #[test]
    fn a_log_of_version_6_gains_the_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sito.db");
        drop(ActionLog::open(&path).unwrap());
        // Takes the file back to the schema before the settings existed.
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("DROP TABLE settings; PRAGMA user_version = 6;")
            .unwrap();
        drop(conn);

        let log = ActionLog::open(&path).unwrap();
        log.set_setting("language", "en").unwrap();
        assert_eq!(log.setting("language").unwrap().as_deref(), Some("en"));
    }
}
