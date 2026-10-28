//! State shared between commands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use sito_core::timeline::{Photo, Timeline, compare_photos};

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
    /// Ids of removed photos once a refresh found them in the rejected folder
    /// (and so no longer needs them in `removed`), for restoring them.
    removed_ids: HashMap<PathBuf, usize>,
}

impl Scan {
    pub(crate) fn new(id: u64, timeline: Timeline) -> Self {
        let count = timeline.photos.len();
        Self {
            id,
            timeline,
            ids: (0..count).collect(),
            next_id: count,
            removed: HashMap::new(),
            removed_ids: HashMap::new(),
        }
    }

    /// Replaces the timeline with a newer scan of the same folder, keeping
    /// the scan id and the ids of photos that are still there (by path), so
    /// thumbnail URLs and the UI's references stay valid. New photos get new
    /// ids; photos that are gone are dropped.
    pub fn refresh(&mut self, timeline: Timeline) {
        let mut old: HashMap<PathBuf, usize> = self
            .photos()
            .map(|(id, photo)| (photo.path.clone(), id))
            .collect();
        // Rejected photos are now in the new timeline's rejected list.
        for (path, (id, _)) in self.removed.drain() {
            self.removed_ids.insert(path, id);
        }
        let mut ids = Vec::with_capacity(timeline.photos.len());
        for photo in &timeline.photos {
            let id = old
                .remove(&photo.path)
                .or_else(|| self.removed_ids.remove(&photo.path))
                .unwrap_or_else(|| {
                    self.next_id += 1;
                    self.next_id - 1
                });
            ids.push(id);
        }
        self.ids = ids;
        self.timeline = timeline;
    }

    /// The photos with their ids, in timeline order.
    pub fn photos(&self) -> impl Iterator<Item = (usize, &Photo)> {
        self.ids.iter().copied().zip(&self.timeline.photos)
    }

    /// The timeline's photos together with the rejected ones (in "Odrzucone"
    /// folders when scanned, or removed from this scan since), in timeline
    /// order, and for each whether it was rejected. Series are found on this
    /// list, so rejecting photos never breaks a series up.
    pub fn photos_with_removed(&self) -> (Vec<Photo>, Vec<bool>) {
        let mut all: Vec<(Photo, bool)> = self
            .timeline
            .photos
            .iter()
            .map(|photo| (photo.clone(), false))
            .chain(
                self.timeline
                    .rejected
                    .iter()
                    .map(|photo| (photo.clone(), true)),
            )
            .chain(
                self.removed
                    .values()
                    .map(|(_, photo)| (photo.clone(), true)),
            )
            .collect();
        all.sort_by(|a, b| compare_photos(&a.0, &b.0));
        all.into_iter().unzip()
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

    /// Puts the photo moved back from `rejected_path` to `path` on the
    /// timeline and returns its id: the id it had if it was removed from this
    /// scan, otherwise a new one, with the photo read by `read` (it was
    /// rejected before this scan, so it is taken off the rejected list).
    pub fn restore(
        &mut self,
        path: &Path,
        rejected_path: &Path,
        read: impl FnOnce(&Path) -> Option<Photo>,
    ) -> Option<usize> {
        let (id, photo) = if let Some(known) = self.removed.remove(path) {
            known
        } else {
            self.timeline
                .rejected
                .retain(|photo| photo.path != rejected_path);
            let photo = read(path)?;
            let id = self.removed_ids.remove(path).unwrap_or_else(|| {
                self.next_id += 1;
                self.next_id - 1
            });
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

    /// Refreshes scan `scan_id` with a newer `timeline` of its folder (see
    /// [`Scan::refresh`]). Returns `false`, changing nothing, if another scan
    /// has replaced it in the meantime.
    pub fn refresh(&self, scan_id: u64, timeline: Timeline) -> bool {
        match self.scan().as_mut() {
            Some(scan) if scan.id == scan_id => {
                scan.refresh(timeline);
                true
            }
            _ => false,
        }
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
            rejected: vec![],
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
            sidecar: None,
            companion: None,
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

        assert_eq!(
            scan.restore(Path::new("b.jpg"), Path::new("Odrzucone/b.jpg"), |_| None),
            Some(1)
        );
        assert_eq!(
            scan.restore(Path::new("a.jpg"), Path::new("Odrzucone/a.jpg"), |_| None),
            Some(0)
        );

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
        let mut found = timeline(&["a.jpg", "c.jpg"]);
        found.rejected = vec![photo("Odrzucone/b.jpg", 1)];
        let mut scan = Scan::new(1, found);
        let older = photo("b.jpg", 1);

        let id = scan.restore(Path::new("b.jpg"), Path::new("Odrzucone/b.jpg"), |_| {
            Some(older)
        });

        assert_eq!(id, Some(2));
        assert_eq!(scan.timeline.rejected, Vec::new());
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

        assert_eq!(
            scan.restore(
                Path::new("gone.jpg"),
                Path::new("Odrzucone/gone.jpg"),
                |_| None
            ),
            None
        );
        assert_eq!(names(&scan), [(0, "a.jpg".into())]);
    }

    #[test]
    fn rejected_photos_are_listed_in_place_for_finding_series() {
        let mut found = timeline(&["a.jpg", "b.jpg", "d.jpg"]);
        // Rejected in an earlier session: found in "Odrzucone" by the scan.
        found.rejected = vec![photo("Odrzucone/c.jpg", 2)];
        found.photos[2] = photo("d.jpg", 3);
        let mut scan = Scan::new(1, found);
        // Rejected now.
        scan.remove(1);

        let (all, removed) = scan.photos_with_removed();

        let paths: Vec<_> = all
            .iter()
            .map(|p| p.path.to_string_lossy().into_owned())
            .collect();
        assert_eq!(paths, ["a.jpg", "b.jpg", "Odrzucone/c.jpg", "d.jpg"]);
        assert_eq!(removed, [false, true, true, false]);
    }

    #[test]
    fn a_refresh_keeps_ids_of_photos_still_there_and_numbers_new_ones() {
        let mut scan = Scan::new(7, timeline(&["a.jpg", "b.jpg", "c.jpg"]));

        // b.jpg is gone; d.jpg is new and the oldest.
        let mut newer = timeline(&["a.jpg", "c.jpg"]);
        newer.photos.push(photo("d.jpg", 9));
        scan.refresh(newer);

        assert_eq!(scan.id, 7);
        assert_eq!(
            names(&scan),
            [
                (0, "a.jpg".into()),
                (2, "c.jpg".into()),
                (3, "d.jpg".into())
            ]
        );
        assert!(scan.photo(1).is_none());
    }

    #[test]
    fn a_photo_rejected_before_a_refresh_is_restored_with_its_id() {
        let mut scan = Scan::new(7, timeline(&["a.jpg", "b.jpg"]));
        scan.remove(1);
        // The refresh finds b.jpg in the rejected folder.
        let mut newer = timeline(&["a.jpg"]);
        newer.rejected = vec![photo("Odrzucone/b.jpg", 1)];
        scan.refresh(newer);
        let (all, _) = scan.photos_with_removed();
        assert_eq!(all.len(), 2, "no duplicate of the rejected photo");

        let id = scan.restore(Path::new("b.jpg"), Path::new("Odrzucone/b.jpg"), |_| {
            Some(photo("b.jpg", 1))
        });

        assert_eq!(id, Some(1));
        assert_eq!(scan.timeline.rejected, Vec::new());
    }

    #[test]
    fn a_refresh_of_a_replaced_scan_changes_nothing() {
        let state = AppState::default();
        let old = state.set_timeline(timeline(&["a.jpg"]));
        let current = state.set_timeline(timeline(&["b.jpg"]));

        assert!(!state.refresh(old, timeline(&["c.jpg"])));
        assert!(state.refresh(current, timeline(&["b.jpg", "d.jpg"])));
        assert_eq!(
            state.photo(current, 1).map(|p| p.path),
            Some(PathBuf::from("d.jpg"))
        );
    }
}
