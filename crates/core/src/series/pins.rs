//! Series edited by hand: photos pinned to a manual series or as single
//! photos, by path. Pinned photos keep their place whatever the gap; the
//! other photos are grouped automatically around them.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use super::SeriesKind;
use crate::timeline::Photo;

/// Identifies a series edited by hand. Unique across all folders, so series
/// of a folder and of its subfolder never mix when both are opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ManualSeriesId(pub i64);

/// Where an edit put a photo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pin {
    /// In the manual series with this id.
    Series(ManualSeriesId),
    /// On its own, outside any series.
    Single,
}

/// The pinned photos of a folder, by path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pins {
    by_path: HashMap<PathBuf, Pin>,
}

/// How an edit changes the pins: the pins to set (replacing a pin the photo
/// had) and the photos to unpin.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PinChanges {
    /// Photos and their new pins.
    pub set: Vec<(PathBuf, Pin)>,
    /// Photos to unpin, so they are grouped automatically again.
    pub clear: Vec<PathBuf>,
}

impl Pins {
    /// The pin of the photo at `path`, if it has one.
    #[must_use]
    pub fn get(&self, path: &Path) -> Option<Pin> {
        self.by_path.get(path).copied()
    }

    /// Whether no photo is pinned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_path.is_empty()
    }

    /// The paths pinned to manual series `id`, in no particular order.
    pub fn paths_in(&self, id: ManualSeriesId) -> impl Iterator<Item = &Path> {
        self.by_path
            .iter()
            .filter(move |(_, pin)| **pin == Pin::Series(id))
            .map(|(path, _)| path.as_path())
    }

    /// Applies `changes`: first the pins to clear, then the pins to set.
    pub fn apply(&mut self, changes: &PinChanges) {
        for path in &changes.clear {
            self.by_path.remove(path);
        }
        for (path, pin) in &changes.set {
            self.by_path.insert(path.clone(), *pin);
        }
    }
}

impl FromIterator<(PathBuf, Pin)> for Pins {
    fn from_iter<I: IntoIterator<Item = (PathBuf, Pin)>>(iter: I) -> Self {
        Self {
            by_path: iter.into_iter().collect(),
        }
    }
}

/// The manual series among the pins `pin_of` (one per photo of a timeline),
/// as lists of positions in timeline order, by id.
pub(super) fn manual_groups(pin_of: &[Option<Pin>]) -> Vec<(Vec<usize>, SeriesKind)> {
    let mut groups: BTreeMap<ManualSeriesId, Vec<usize>> = BTreeMap::new();
    for (position, pin) in pin_of.iter().enumerate() {
        if let Some(Pin::Series(id)) = pin {
            groups.entry(*id).or_default().push(position);
        }
    }
    groups
        .into_iter()
        .map(|(id, members)| (members, SeriesKind::Manual(id)))
        .collect()
}

/// Positions of the photos of `all` not marked `true` in `removed`, counted
/// among those, that are pinned as single photos.
pub(super) fn pinned_singles(all: &[Photo], removed: &[bool], pins: &Pins) -> Vec<usize> {
    if pins.is_empty() {
        return Vec::new();
    }
    all.iter()
        .zip(removed)
        .filter(|(_, gone)| !**gone)
        .enumerate()
        .filter(|(_, (photo, _))| pins.get(&photo.path) == Some(Pin::Single))
        .map(|(position, _)| position)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::tests::shots;
    use super::super::{DEFAULT_MAX_GAP, Grouping, SeriesLabel, group};
    use super::*;

    const A: Option<&str> = Some("Canon EOS R6");
    const B: Option<&str> = Some("Pixel 8");

    /// Photos of one camera at the given seconds after noon, newest first.
    fn photos(seconds: &[i64]) -> Vec<Photo> {
        shots(&seconds.iter().map(|&s| (s, A)).collect::<Vec<_>>())
    }

    /// Pins for the photos taken at the given seconds (named `<s>.jpg`).
    fn pins(pinned: &[(i64, Pin)]) -> Pins {
        pinned
            .iter()
            .map(|&(s, pin)| (PathBuf::from(format!("{s}.jpg")), pin))
            .collect()
    }

    fn manual(id: i64) -> Pin {
        Pin::Series(ManualSeriesId(id))
    }

    fn members(photos: &[Photo], gap: u64, pins: &Pins) -> Vec<Vec<usize>> {
        group(photos, Duration::from_secs(gap), pins)
            .into_iter()
            .map(|s| s.members)
            .collect()
    }

    #[test]
    fn without_pins_grouping_is_automatic() {
        // Newest first: 30, 6, 3, 0
        let photos = photos(&[0, 3, 6, 30]);

        let series = group(&photos, DEFAULT_MAX_GAP, &Pins::default());

        assert_eq!(series.len(), 1);
        assert_eq!(series[0].members, [1, 2, 3]);
        assert_eq!(series[0].kind, SeriesKind::Automatic);
    }

    #[test]
    fn a_manual_series_stays_as_it_is_with_any_gap() {
        // Newest first: 40, 20, 6, 3, 0; 20 and 3 are one manual series.
        let photos = photos(&[0, 3, 6, 20, 40]);
        let pins = pins(&[(20, manual(7)), (3, manual(7))]);

        let series = group(&photos, Duration::from_secs(1), &pins);
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].members, [1, 3]);
        assert_eq!(series[0].kind, SeriesKind::Manual(ManualSeriesId(7)));

        // With a large gap the others would form a series, but never across
        // the pinned photos between them.
        assert_eq!(members(&photos, 60, &pins), [[1, 3]]);
    }

    #[test]
    fn unpinned_photos_never_form_a_series_around_a_pinned_one() {
        // Newest first: 10, 5, 0; 5 is pinned on its own.
        let photos = photos(&[0, 5, 10]);
        let pins = pins(&[(5, Pin::Single)]);

        assert_eq!(members(&photos, 60, &pins), Vec::<Vec<usize>>::new());
    }

    #[test]
    fn a_pinned_single_splits_a_burst_at_every_gap() {
        // Newest first: 8, 6, 4, 2, 0; 4 is pinned on its own.
        let photos = photos(&[0, 2, 4, 6, 8]);
        let pins = pins(&[(4, Pin::Single)]);

        for gap in [2, 10, 60] {
            assert_eq!(members(&photos, gap, &pins), [vec![0, 1], vec![3, 4]]);
        }
    }

    #[test]
    fn a_pinned_photo_is_a_barrier_only_for_its_own_camera() {
        // Newest first: a@6, b@4, a@2, b@0; b@4 is pinned on its own.
        let photos = shots(&[(0, B), (2, A), (4, B), (6, A)]);
        let pins = pins(&[(4, Pin::Single)]);

        assert_eq!(members(&photos, 10, &pins), [[0, 2]]);
    }

    #[test]
    fn manual_and_automatic_series_are_sorted_by_their_newest_photo() {
        // Newest first: 30, 28, 10, 8, 2, 0; 10 and 2 are a manual series.
        let photos = photos(&[0, 2, 8, 10, 28, 30]);
        let pins = pins(&[(10, manual(1)), (2, manual(1))]);

        assert_eq!(members(&photos, 3, &pins), [vec![0, 1], vec![2, 4]]);
    }

    #[test]
    fn a_manual_series_may_mix_cameras_and_is_labelled_with_the_others() {
        // Newest first: b@8, a@6, b@4, a@2; a@6 and b@4 are a manual series.
        let photos = shots(&[(2, A), (4, B), (6, A), (8, B)]);
        let pins = pins(&[(6, manual(1)), (4, manual(1))]);

        let series = group(&photos, Duration::from_secs(60), &pins);

        // b@8 and a@2 stay single: each is the only unpinned photo of its
        // camera on its side of the manual series.
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].members, [1, 2]);
        assert_eq!(series[0].label, SeriesLabel::Plain);
    }

    #[test]
    fn labels_cover_manual_and_automatic_series_together() {
        // a: 0, 2 (automatic); b: 1, 3 pinned as a manual series.
        let photos = shots(&[(0, A), (1, B), (2, A), (3, B)]);
        let pins = pins(&[(3, manual(1)), (1, manual(1))]);

        let labels: Vec<SeriesLabel> = group(&photos, DEFAULT_MAX_GAP, &pins)
            .into_iter()
            .map(|s| s.label)
            .collect();

        assert_eq!(
            labels,
            [
                SeriesLabel::Camera("Pixel 8".to_owned()),
                SeriesLabel::Camera("Canon EOS R6".to_owned()),
            ]
        );
    }

    #[test]
    fn pins_of_photos_not_on_the_timeline_are_ignored() {
        let photos = photos(&[0, 3]);
        let pins = pins(&[(99, manual(1)), (98, Pin::Single)]);

        assert_eq!(members(&photos, 10, &pins), [[0, 1]]);
    }

    #[test]
    fn a_manual_series_keeps_its_rejected_photos_and_shows_the_rest() {
        // Newest first: 6, 3, 0; all three pinned to one series, 3 rejected.
        let photos = photos(&[0, 3, 6]);
        let pins = pins(&[(6, manual(1)), (3, manual(1)), (0, manual(1))]);

        let grouping = Grouping::new(&photos, &[false, true, false], DEFAULT_MAX_GAP, &pins);

        assert_eq!(grouping.series.len(), 1);
        assert_eq!(grouping.series[0].members, [0, 1]);
    }

    #[test]
    fn a_manual_series_with_one_photo_left_is_a_series_of_one() {
        let photos = photos(&[0, 30]);
        let pins = pins(&[(30, manual(1))]);

        assert_eq!(members(&photos, 10, &pins), [[0]]);
    }

    #[test]
    fn the_grouping_lists_the_pinned_single_photos_left() {
        // Newest first: 9, 6, 3, 0; 6 (rejected) and 3 pinned on their own.
        let photos = photos(&[0, 3, 6, 9]);
        let pins = pins(&[(6, Pin::Single), (3, Pin::Single)]);

        let grouping = Grouping::new(
            &photos,
            &[false, true, false, false],
            DEFAULT_MAX_GAP,
            &pins,
        );

        // Left: 9, 3, 0; 3 is at position 1 among them.
        assert_eq!(grouping.pinned_singles, [1]);
    }

    #[test]
    fn applying_changes_clears_then_sets() {
        let mut pins = pins(&[(1, Pin::Single), (2, manual(1))]);

        pins.apply(&PinChanges {
            set: vec![(PathBuf::from("1.jpg"), manual(2))],
            clear: vec![PathBuf::from("1.jpg"), PathBuf::from("2.jpg")],
        });

        assert_eq!(pins.get(Path::new("1.jpg")), Some(manual(2)));
        assert_eq!(pins.get(Path::new("2.jpg")), None);
    }

    #[test]
    fn lists_the_paths_of_a_manual_series() {
        let pins = pins(&[(1, manual(1)), (2, manual(2)), (3, manual(1))]);

        let mut paths: Vec<&Path> = pins.paths_in(ManualSeriesId(1)).collect();
        paths.sort();

        assert_eq!(paths, [Path::new("1.jpg"), Path::new("3.jpg")]);
    }
}
