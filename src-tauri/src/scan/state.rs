//! State shared between commands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sito_core::series::Grouping;
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
    /// Counts the changes to the timeline, so work done on a copy of it can
    /// tell whether it is still up to date.
    revision: u64,
    /// The series of the current revision for one gap, found last.
    series: Option<(Duration, Arc<Grouping>)>,
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
            revision: 0,
            series: None,
        }
    }

    /// Changes with every change to the timeline (a refresh, updated photos,
    /// a removed or restored photo).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The series for `max_gap` stored with [`Scan::store_series`], if the
    /// timeline has not changed since.
    pub fn cached_series(&self, max_gap: Duration) -> Option<Arc<Grouping>> {
        self.series
            .as_ref()
            .filter(|(gap, _)| *gap == max_gap)
            .map(|(_, series)| Arc::clone(series))
    }

    /// Keeps `series`, found for `max_gap` on the timeline at `revision`,
    /// unless the timeline has changed since.
    pub fn store_series(&mut self, revision: u64, max_gap: Duration, series: Arc<Grouping>) {
        if revision == self.revision {
            self.series = Some((max_gap, series));
        }
    }

    /// Records a change to the timeline: a new revision without cached series.
    fn changed(&mut self) {
        self.revision += 1;
        self.series = None;
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
        self.changed();
    }

    /// Replaces the photos that have the paths of `read` with those (e.g.
    /// once their EXIF is read), keeping their ids, and puts the timeline
    /// back in order, as their capture times may have changed.
    pub fn update_photos(&mut self, read: &[Photo]) {
        let read: HashMap<&Path, &Photo> = read
            .iter()
            .map(|photo| (photo.path.as_path(), photo))
            .collect();
        let removed = self.removed.values_mut().map(|(_, photo)| photo);
        for photo in self
            .timeline
            .photos
            .iter_mut()
            .chain(&mut self.timeline.rejected)
            .chain(removed)
        {
            if let Some(&newer) = read.get(photo.path.as_path()) {
                photo.clone_from(newer);
            }
        }
        let mut entries: Vec<(usize, Photo)> = self
            .ids
            .drain(..)
            .zip(self.timeline.photos.drain(..))
            .collect();
        entries.sort_by(|a, b| compare_photos(&a.1, &b.1));
        (self.ids, self.timeline.photos) = entries.into_iter().unzip();
        self.timeline.rejected.sort_by(compare_photos);
        self.changed();
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
        self.timeline.photos.get(self.position_of(id)?)
    }

    /// Number of photos on the timeline.
    pub fn photo_count(&self) -> usize {
        self.ids.len()
    }

    /// Position on the timeline of the photo with id `id`, if it is on it.
    pub fn position_of(&self, id: usize) -> Option<usize> {
        self.ids.iter().position(|&other| other == id)
    }

    /// The id and photo at timeline position `position`.
    pub fn entry_at(&self, position: usize) -> Option<(usize, &Photo)> {
        Some((
            *self.ids.get(position)?,
            self.timeline.photos.get(position)?,
        ))
    }

    /// Takes the photo with id `id` off the timeline, remembering it for
    /// [`Scan::restore`]; returns it.
    pub fn remove(&mut self, id: usize) -> Option<Photo> {
        let position = self.position_of(id)?;
        self.ids.remove(position);
        let photo = self.timeline.photos.remove(position);
        self.removed.insert(photo.path.clone(), (id, photo.clone()));
        self.changed();
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
        self.changed();
        Some(id)
    }
}

/// Which opening of a folder some background work belongs to; work of an
/// earlier one stops (see [`AppState::is_latest`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Generation(u64);

/// App-wide state managed by Tauri.
#[derive(Debug, Default)]
pub struct AppState {
    scan: Mutex<Option<Scan>>,
    /// How many times a folder has been opened.
    opened: AtomicU64,
}

impl AppState {
    /// Starts opening a folder: from now on, background work of folders
    /// opened before is no longer [`Self::is_latest`].
    pub fn begin_scan(&self) -> Generation {
        Generation(self.opened.fetch_add(1, Ordering::SeqCst) + 1)
    }

    /// Whether no folder has been opened since `generation`.
    pub fn is_latest(&self, generation: Generation) -> bool {
        self.opened.load(Ordering::SeqCst) == generation.0
    }

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

    /// Updates photos of scan `scan_id` (see [`Scan::update_photos`]).
    /// Returns `false`, changing nothing, if another scan has replaced it.
    pub fn update_photos(&self, scan_id: u64, read: &[Photo]) -> bool {
        match self.scan().as_mut() {
            Some(scan) if scan.id == scan_id => {
                scan.update_photos(read);
                true
            }
            _ => false,
        }
    }

    /// Whether scan `scan_id` is the one shown.
    pub fn is_current(&self, scan_id: u64) -> bool {
        self.scan().as_ref().is_some_and(|scan| scan.id == scan_id)
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
    use sito_core::series::{Arrangement, Series, SeriesLabel};
    use sito_core::timeline::{ExifInfo, TimeSource};

    use super::*;

    #[test]
    fn opening_a_folder_makes_earlier_work_outdated() {
        let state = AppState::default();
        let first = state.begin_scan();
        assert!(state.is_latest(first));

        let second = state.begin_scan();

        assert!(!state.is_latest(first));
        assert!(state.is_latest(second));
    }

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
            exif: ExifInfo::Read {
                orientation: Orientation::NORMAL,
                camera: None,
            },
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

    fn cached_lens(scan: &Scan, max_gap: Duration) -> Option<Vec<usize>> {
        scan.cached_series(max_gap)
            .map(|grouping| grouping.series.iter().map(|s| s.members.len()).collect())
    }

    fn store_one_series(scan: &mut Scan, max_gap: Duration) {
        scan.store_series(
            scan.revision(),
            max_gap,
            Arc::new(Grouping {
                series: vec![Series {
                    members: vec![0, 1],
                    label: SeriesLabel::Plain,
                }],
                arrangement: Arrangement::default(),
            }),
        );
    }

    #[test]
    fn series_are_cached_for_their_gap() {
        let gap = Duration::from_secs(10);
        let mut scan = Scan::new(1, timeline(&["a.jpg", "b.jpg"]));
        assert_eq!(cached_lens(&scan, gap), None);

        store_one_series(&mut scan, gap);

        assert_eq!(cached_lens(&scan, gap), Some(vec![2]));
        assert_eq!(cached_lens(&scan, Duration::from_secs(5)), None);
    }

    #[test]
    fn series_found_on_an_older_revision_are_not_stored() {
        let gap = Duration::from_secs(10);
        let mut scan = Scan::new(1, timeline(&["a.jpg", "b.jpg", "c.jpg"]));
        let before = scan.revision();
        scan.remove(1);

        scan.store_series(before, gap, Arc::new(Grouping::default()));

        assert_eq!(cached_lens(&scan, gap), None);
    }

    #[test]
    fn every_change_to_the_timeline_drops_the_cached_series() {
        let gap = Duration::from_secs(10);
        let mut scan = Scan::new(1, timeline(&["a.jpg", "b.jpg", "c.jpg"]));
        let changes: [&dyn Fn(&mut Scan); 4] = [
            &|scan| {
                scan.remove(1);
            },
            &|scan| {
                scan.restore(Path::new("b.jpg"), Path::new("x/b.jpg"), |_| None);
            },
            &|scan| scan.update_photos(&[photo("a.jpg", 5)]),
            &|scan| scan.refresh(timeline(&["a.jpg"])),
        ];
        for change in changes {
            store_one_series(&mut scan, gap);
            let before = scan.revision();

            change(&mut scan);

            assert!(scan.revision() > before);
            assert_eq!(cached_lens(&scan, gap), None);
        }
    }

    #[test]
    fn photos_are_found_by_position_and_id() {
        let mut scan = Scan::new(1, timeline(&["a.jpg", "b.jpg", "c.jpg"]));
        scan.remove(0);

        assert_eq!(scan.photo_count(), 2);
        assert_eq!(scan.position_of(2), Some(1));
        assert_eq!(scan.position_of(0), None);
        let (id, photo) = scan.entry_at(0).unwrap();
        assert_eq!((id, photo.path.as_path()), (1, Path::new("b.jpg")));
        assert!(scan.entry_at(2).is_none());
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

    #[test]
    fn updated_photos_move_to_their_new_time_and_keep_their_ids() {
        let mut scan = Scan::new(7, timeline(&["a.jpg", "b.jpg", "c.jpg"]));
        // Its EXIF says c.jpg is the newest.
        let newer_c = photo("c.jpg", 0).taken_at + chrono::Duration::seconds(5);

        scan.update_photos(&[Photo {
            taken_at: newer_c,
            ..photo("c.jpg", 2)
        }]);

        assert_eq!(
            names(&scan),
            [
                (2, "c.jpg".to_owned()),
                (0, "a.jpg".to_owned()),
                (1, "b.jpg".to_owned())
            ]
        );
        assert_eq!(scan.photo(2).map(|p| p.taken_at), Some(newer_c));
    }

    #[test]
    fn updates_reach_removed_photos_too() {
        let mut scan = Scan::new(7, timeline(&["a.jpg", "b.jpg"]));
        let removed = scan.remove(1).unwrap();
        let read = Photo {
            size: 99,
            ..removed
        };

        scan.update_photos(std::slice::from_ref(&read));
        let (all, _) = scan.photos_with_removed();

        assert!(all.contains(&read));
    }

    #[test]
    fn updates_of_a_replaced_scan_change_nothing() {
        let state = AppState::default();
        let old = state.set_timeline(timeline(&["a.jpg"]));
        let current = state.set_timeline(timeline(&["a.jpg"]));

        assert!(!state.is_current(old));
        assert!(state.is_current(current));
        assert!(!state.update_photos(old, &[photo("a.jpg", 9)]));
        assert_eq!(state.photo(current, 0), Some(photo("a.jpg", 0)));
    }
}
