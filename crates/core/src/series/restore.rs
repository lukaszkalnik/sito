//! What „Przywróć automatyczne grupowanie” unpins: the whole stretch of edited
//! photos around a photo, so taking a photo out of a series (or cutting one)
//! can be taken back from any of the photos the edit touched.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use super::camera_of;
use super::pins::{ManualSeriesId, Pin, Pins};
use crate::timeline::Photo;

/// The paths to unpin when the pinned photo at `position` of `photos`
/// (sorted newest first) is restored: the run of pinned photos around it in
/// its camera's chain, every photo of a manual series one of them belongs to
/// (also photos of it that are gone for now, so they do not bring it back
/// when they return), and so on until nothing more is reached. `None` if the
/// photo is not pinned.
pub(super) fn restored_paths(
    photos: &[Photo],
    pins: &Pins,
    position: usize,
) -> Option<Vec<PathBuf>> {
    pins.get(&photos.get(position)?.path)?;
    Some(stretch_of(photos, pins, [position]))
}

/// The stretch of edited photos (see [`restored_paths`]) around the pinned
/// photos among `positions`, as sorted paths.
pub(super) fn stretch_of(
    photos: &[Photo],
    pins: &Pins,
    positions: impl IntoIterator<Item = usize>,
) -> Vec<PathBuf> {
    let pin_of: Vec<Option<Pin>> = photos.iter().map(|p| pins.get(&p.path)).collect();
    let neighbours = chain_neighbours(photos);
    let is_pinned = |p: usize| pin_of.get(p).copied().flatten().is_some();
    let mut todo: Vec<usize> = positions.into_iter().filter(|&p| is_pinned(p)).collect();
    let mut reached: BTreeSet<usize> = todo.iter().copied().collect();
    let mut series: BTreeSet<ManualSeriesId> = BTreeSet::new();
    while let Some(current) = todo.pop() {
        let (before, after) = neighbours.get(current).copied().unwrap_or_default();
        let mut next: Vec<usize> = [before, after].into_iter().flatten().collect();
        if let Some(Some(Pin::Series(id))) = pin_of.get(current)
            && series.insert(*id)
        {
            next.extend(members_of(&pin_of, *id));
        }
        for candidate in next {
            if is_pinned(candidate) && reached.insert(candidate) {
                todo.push(candidate);
            }
        }
    }
    let mut paths: Vec<PathBuf> = reached.iter().map(|&p| photos[p].path.clone()).collect();
    for id in series {
        paths.extend(pins.paths_in(id).map(Path::to_path_buf));
    }
    paths.sort();
    paths.dedup();
    paths
}

/// For each photo, the positions of the photos of its camera just before and
/// after it (newer and older), if any.
fn chain_neighbours(photos: &[Photo]) -> Vec<(Option<usize>, Option<usize>)> {
    let mut neighbours = vec![(None, None); photos.len()];
    let mut last: HashMap<Option<&str>, usize> = HashMap::new();
    for (position, photo) in photos.iter().enumerate() {
        if let Some(previous) = last.insert(camera_of(photo), position) {
            neighbours[previous].1 = Some(position);
            neighbours[position].0 = Some(previous);
        }
    }
    neighbours
}

/// Positions of the photos pinned to manual series `id`.
fn members_of(pin_of: &[Option<Pin>], id: ManualSeriesId) -> impl Iterator<Item = usize> + '_ {
    pin_of
        .iter()
        .enumerate()
        .filter(move |(_, pin)| **pin == Some(Pin::Series(id)))
        .map(|(position, _)| position)
}

#[cfg(test)]
mod tests {
    use super::super::tests::shots;
    use super::*;

    const A: Option<&str> = Some("Canon EOS R6");
    const B: Option<&str> = Some("Pixel 8");

    fn pins(pinned: &[(i64, Pin)]) -> Pins {
        pinned
            .iter()
            .map(|&(s, pin)| (PathBuf::from(format!("{s}.jpg")), pin))
            .collect()
    }

    fn names(paths: &[PathBuf]) -> Vec<String> {
        paths.iter().map(|p| p.display().to_string()).collect()
    }

    const ONE: Pin = Pin::Series(ManualSeriesId(1));

    #[test]
    fn a_photo_taken_out_comes_back_with_the_rest_of_its_series() {
        // Newest first: 6, 4, 2, 0; 4 was taken out, 6, 2 and 0 kept together.
        let photos = shots(&[(0, A), (2, A), (4, A), (6, A)]);
        let pins = pins(&[(6, ONE), (4, Pin::Single), (2, ONE), (0, ONE)]);

        let paths = restored_paths(&photos, &pins, 1).unwrap();

        assert_eq!(names(&paths), ["0.jpg", "2.jpg", "4.jpg", "6.jpg"]);
    }

    #[test]
    fn the_stretch_ends_at_an_unpinned_photo_of_the_camera() {
        // Newest first: 8, 6, 4, 2, 0; 8 and 2 pinned, 4 not.
        let photos = shots(&[(0, A), (2, A), (4, A), (6, A), (8, A)]);
        let pins = pins(&[(8, Pin::Single), (6, Pin::Single), (2, Pin::Single)]);

        let paths = restored_paths(&photos, &pins, 0).unwrap();

        assert_eq!(names(&paths), ["6.jpg", "8.jpg"]);
    }

    #[test]
    fn other_cameras_photos_in_between_do_not_end_the_stretch() {
        // Newest first: a@6, b@4, a@2; both a photos pinned on their own.
        let photos = shots(&[(2, A), (4, B), (6, A)]);
        let pins = pins(&[(6, Pin::Single), (2, Pin::Single)]);

        let paths = restored_paths(&photos, &pins, 0).unwrap();

        assert_eq!(names(&paths), ["2.jpg", "6.jpg"]);
    }

    #[test]
    fn a_manual_series_brings_its_photos_that_are_gone_and_of_other_cameras() {
        // a@4 and b@0 are one manual series, 99 is gone for now.
        let photos = shots(&[(0, B), (2, A), (4, A)]);
        let pins = pins(&[(4, ONE), (0, ONE), (99, ONE)]);

        let paths = restored_paths(&photos, &pins, 0).unwrap();

        assert_eq!(names(&paths), ["0.jpg", "4.jpg", "99.jpg"]);
    }

    #[test]
    fn an_unpinned_photo_has_nothing_to_restore() {
        let photos = shots(&[(0, A)]);

        assert_eq!(restored_paths(&photos, &Pins::default(), 0), None);
    }
}
