//! State shared between commands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use sito_core::timeline::{Photo, Timeline};

/// The result of one finished scan.
#[derive(Debug)]
pub struct Scan {
    /// Unique across app launches (milliseconds since the Unix epoch, bumped
    /// if needed), so the UI can use it as a cache-buster in thumbnail URLs.
    pub id: u64,
    /// Photos found by the scan, minus rejected ones.
    pub timeline: Timeline,
    /// Id of each photo in `timeline.photos`, same order. A photo keeps its
    /// id while photos before it are removed or restored, so thumbnail URLs
    /// and the UI's references stay valid.
    ids: Vec<usize>,
    /// Next id for a photo that was not part of the scan.
    next_id: usize,
    /// Photos removed from this scan, by path, with their id, so restoring
    /// one brings it back with the same id.
    removed: HashMap<PathBuf, (usize, Photo)>,
}

impl Scan {
    fn new(id: u64, timeline: Timeline) -> Self {
        let count = timeline.photos.len();
        Self {
            id,
            timeline,
            ids: (0..count).collect(),
            next_id: count,
            removed: HashMap::new(),
        }
    }

    /// The photos with their ids, in timeline order.
    pub fn photos(&self) -> impl Iterator<Item = (usize, &Photo)> {
        self.ids.iter().copied().zip(&self.timeline.photos)
    }

    /// The photo with id `id`, if it is on the timeline.
    pub fn photo(&self, id: usize) -> Option<&Photo> {
        let position = self.ids.iter().position(|&other| other == id)?;
        self.timeline.photos.get(position)
    }

    /// Takes the photo with id `id` off the timeline, remembering it for
    /// [`Scan::restore`]; returns it.
    pub fn remove(&mut self, id: usize) -> Option<Photo> {
        let position = self.ids.iter().position(|&other| other == id)?;
        self.ids.remove(position);
        let photo = self.timeline.photos.remove(position);
        self.removed.insert(photo.path.clone(), (id, photo.clone()));
        Some(photo)
    }

    /// Puts the photo at `path` back on the timeline and returns its id: the
    /// id it had if it was removed from this scan, otherwise a new one, with
    /// the photo read by `read` (it was removed before this scan).
    pub fn restore(
        &mut self,
        path: &Path,
        read: impl FnOnce(&Path) -> Option<Photo>,
    ) -> Option<usize> {
        let (id, photo) = if let Some(known) = self.removed.remove(path) {
            known
        } else {
            let photo = read(path)?;
            let id = self.next_id;
            self.next_id += 1;
            (id, photo)
        };
        let position = self.timeline.insert(photo);
        self.ids.insert(position, id);
        Some(id)
    }
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
        *scan = Some(Scan::new(id, timeline));
        id
    }

    /// Photo `id` of scan `scan_id`, or `None` if that scan is no longer
    /// current or has no such photo.
    pub fn photo(&self, scan_id: u64, id: usize) -> Option<Photo> {
        self.scan()
            .as_ref()
            .filter(|scan| scan.id == scan_id)
            .and_then(|scan| scan.photo(id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use sito_core::metadata::Orientation;
    use sito_core::timeline::TimeSource;

    use super::*;

    /// Photos named `names`, one second apart, newest first.
    fn timeline(names: &[&str]) -> Timeline {
        Timeline {
            root: PathBuf::from("gallery"),
            photos: names
                .iter()
                .enumerate()
                .map(|(i, name)| photo(name, i))
                .collect(),
            unreadable_entries: 0,
        }
    }

    fn photo(name: &str, seconds_before_noon: usize) -> Photo {
        Photo {
            path: PathBuf::from(name),
            size: 1,
            modified: None,
            taken_at: NaiveDate::from_ymd_opt(2024, 6, 1)
                .unwrap()
                .and_hms_opt(11, 59, 59 - u32::try_from(seconds_before_noon).unwrap())
                .unwrap(),
            time_source: TimeSource::Exif,
            orientation: Orientation::NORMAL,
            camera: None,
        }
    }

    fn names(scan: &Scan) -> Vec<(usize, String)> {
        scan.photos()
            .map(|(id, p)| (id, p.path.to_string_lossy().into_owned()))
            .collect()
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

    #[test]
    fn removing_a_photo_keeps_the_ids_of_the_others() {
        let mut scan = Scan::new(1, timeline(&["a.jpg", "b.jpg", "c.jpg"]));

        let removed = scan.remove(1).unwrap();

        assert_eq!(removed.path, PathBuf::from("b.jpg"));
        assert_eq!(names(&scan), [(0, "a.jpg".into()), (2, "c.jpg".into())]);
        assert_eq!(scan.photo(2).unwrap().path, PathBuf::from("c.jpg"));
        assert!(scan.photo(1).is_none());
        assert!(scan.remove(1).is_none());
    }

    #[test]
    fn a_restored_photo_gets_its_id_and_place_back() {
        let mut scan = Scan::new(1, timeline(&["a.jpg", "b.jpg", "c.jpg"]));
        scan.remove(1);
        scan.remove(0);

        assert_eq!(scan.restore(Path::new("b.jpg"), |_| None), Some(1));
        assert_eq!(scan.restore(Path::new("a.jpg"), |_| None), Some(0));

        assert_eq!(
            names(&scan),
            [
                (0, "a.jpg".into()),
                (1, "b.jpg".into()),
                (2, "c.jpg".into())
            ]
        );
    }

    #[test]
    fn a_photo_removed_before_this_scan_gets_a_new_id() {
        let mut scan = Scan::new(1, timeline(&["a.jpg", "c.jpg"]));
        let older = photo("b.jpg", 1);

        let id = scan.restore(Path::new("b.jpg"), |_| Some(older));

        assert_eq!(id, Some(2));
        assert_eq!(
            names(&scan),
            [
                (0, "a.jpg".into()),
                (2, "b.jpg".into()),
                (1, "c.jpg".into())
            ]
        );
        assert_eq!(scan.photo(2).unwrap().path, PathBuf::from("b.jpg"));
    }

    #[test]
    fn an_unreadable_photo_is_not_restored() {
        let mut scan = Scan::new(1, timeline(&["a.jpg"]));

        assert_eq!(scan.restore(Path::new("gone.jpg"), |_| None), None);
        assert_eq!(names(&scan), [(0, "a.jpg".into())]);
    }
}
