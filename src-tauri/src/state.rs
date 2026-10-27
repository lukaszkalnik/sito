//! State shared between commands.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use sito_core::timeline::{Photo, Timeline};

/// The result of one finished scan.
#[derive(Debug)]
pub struct Scan {
    /// Unique across app launches (milliseconds since the Unix epoch, bumped
    /// if needed), so the UI can use it as a cache-buster in thumbnail URLs.
    pub id: u64,
    /// Photos found by the scan.
    pub timeline: Timeline,
}

/// App-wide state managed by Tauri.
#[derive(Debug, Default)]
pub struct AppState {
    scan: Mutex<Option<Scan>>,
}

impl AppState {
    /// The last finished scan, if any.
    pub fn scan(&self) -> MutexGuard<'_, Option<Scan>> {
        // A panic while holding the lock cannot leave the scan half-written
        // (it is only ever replaced as a whole), so a poisoned lock is safe to use.
        self.scan.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Replaces the stored scan with `timeline` and returns the new scan id.
    pub fn set_timeline(&self, timeline: Timeline) -> u64 {
        let mut scan = self.scan();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        let id = scan
            .as_ref()
            .map_or(now, |previous| now.max(previous.id + 1));
        *scan = Some(Scan { id, timeline });
        id
    }

    /// Photo `index` of scan `scan_id`, or `None` if that scan is no longer
    /// current or has no such photo.
    pub fn photo(&self, scan_id: u64, index: usize) -> Option<Photo> {
        self.scan()
            .as_ref()
            .filter(|scan| scan.id == scan_id)
            .and_then(|scan| scan.timeline.photos.get(index).cloned())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::NaiveDate;
    use sito_core::metadata::Orientation;
    use sito_core::timeline::TimeSource;

    use super::*;

    fn timeline(names: &[&str]) -> Timeline {
        Timeline {
            root: PathBuf::from("gallery"),
            photos: names
                .iter()
                .map(|name| Photo {
                    path: PathBuf::from(name),
                    size: 1,
                    modified: None,
                    taken_at: NaiveDate::from_ymd_opt(2024, 6, 1)
                        .unwrap()
                        .and_hms_opt(12, 0, 0)
                        .unwrap(),
                    time_source: TimeSource::Exif,
                    orientation: Orientation::NORMAL,
                    camera: None,
                })
                .collect(),
            unreadable_entries: 0,
        }
    }

    #[test]
    fn every_scan_gets_a_new_id() {
        let state = AppState::default();

        let first = state.set_timeline(timeline(&["a.jpg"]));
        let second = state.set_timeline(timeline(&["a.jpg"]));

        assert!(second > first);
    }

    #[test]
    fn photo_is_looked_up_in_the_current_scan_only() {
        let state = AppState::default();
        let old = state.set_timeline(timeline(&["old.jpg"]));
        let current = state.set_timeline(timeline(&["a.jpg", "b.jpg"]));

        assert_eq!(
            state.photo(current, 1).map(|p| p.path),
            Some(PathBuf::from("b.jpg"))
        );
        assert_eq!(state.photo(current, 2), None);
        assert_eq!(state.photo(old, 0), None);
    }

    #[test]
    fn no_photo_before_the_first_scan() {
        assert_eq!(AppState::default().photo(0, 0), None);
    }
}
