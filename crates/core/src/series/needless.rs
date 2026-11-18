//! Keeping only the pins an edit needs: a part of the edited stretch that
//! automatic grouping would form anyway is not marked as edited.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::group;
use super::pins::{Pin, PinChanges, Pins};
use super::restore::stretch_of;
use crate::timeline::Photo;

/// `changes` (an edit of `pins` on `photos` for `max_gap`) without the pins
/// it does not need. The edit leaves a stretch of edited photos (see
/// [`stretch_of`]) grouped in some way; every part of it (a series, or a
/// single photo) that automatic grouping would also form, with the whole
/// stretch unpinned, is unpinned, as long as the series stay exactly as the
/// edit left them. So joining a photo to a series and taking it out again,
/// or joining a single photo and ungrouping the series, leaves no pencil on
/// what is as it was.
pub(super) fn without_needless_pins(
    photos: &[Photo],
    max_gap: Duration,
    pins: &Pins,
    changes: &PinChanges,
) -> PinChanges {
    let mut after = pins.clone();
    after.apply(changes);
    let touched: HashSet<&Path> = changes.set.iter().map(|(path, _)| path.as_path()).collect();
    let positions = photos
        .iter()
        .enumerate()
        .filter(|(_, photo)| touched.contains(photo.path.as_path()))
        .map(|(position, _)| position);
    let stretch = stretch_of(photos, &after, positions);
    let target = shape(photos, max_gap, &after);
    let free = shape(photos, max_gap, &unpinned(&after, &stretch));
    let units = automatic_parts(photos, &after, &stretch, &target, &free);
    let kept = keep_shape(photos, max_gap, after, &units, &target);
    difference(pins, &kept)
}

/// The parts of the stretch, as paths to unpin, that `free` (automatic
/// grouping without the stretch's pins) forms just as `target` does: a
/// manual series with the same photos as an automatic one, or a photo single
/// in both. A series' paths include its photos that are gone for now.
fn automatic_parts(
    photos: &[Photo],
    pins: &Pins,
    stretch: &[PathBuf],
    target: &[Vec<usize>],
    free: &[Vec<usize>],
) -> Vec<Vec<PathBuf>> {
    let in_stretch: HashSet<&Path> = stretch.iter().map(PathBuf::as_path).collect();
    let pinned_in_stretch = |p: usize| {
        photos
            .get(p)
            .is_some_and(|photo| in_stretch.contains(photo.path.as_path()))
    };
    let in_series = |shape: &[Vec<usize>], p: usize| shape.iter().any(|s| s.contains(&p));
    let series = target
        .iter()
        .filter(|s| s.iter().all(|&p| pinned_in_stretch(p)) && free.contains(s))
        .map(|s| series_paths(photos, pins, s));
    let singles = (0..photos.len())
        .filter(|&p| pinned_in_stretch(p) && !in_series(target, p) && !in_series(free, p))
        .map(|p| vec![photos[p].path.clone()]);
    series.chain(singles).collect()
}

/// The paths of the series with `members`, with the other paths pinned to
/// the same manual series (photos gone for now).
fn series_paths(photos: &[Photo], pins: &Pins, members: &[usize]) -> Vec<PathBuf> {
    let mut paths: BTreeSet<PathBuf> = members.iter().map(|&p| photos[p].path.clone()).collect();
    let ids: BTreeSet<_> = paths
        .iter()
        .filter_map(|path| match pins.get(path) {
            Some(Pin::Series(id)) => Some(id),
            Some(Pin::Single) | None => None,
        })
        .collect();
    for id in ids {
        paths.extend(pins.paths_in(id).map(Path::to_path_buf));
    }
    paths.into_iter().collect()
}

/// `pins` without the pins of `units`: all at once if the series stay
/// `target`, else one unit after another, each only if they still do.
fn keep_shape(
    photos: &[Photo],
    max_gap: Duration,
    pins: Pins,
    units: &[Vec<PathBuf>],
    target: &[Vec<usize>],
) -> Pins {
    let all: Vec<PathBuf> = units.concat();
    let at_once = unpinned(&pins, &all);
    if shape(photos, max_gap, &at_once) == target {
        return at_once;
    }
    units.iter().fold(pins, |kept, unit| {
        let trial = unpinned(&kept, unit);
        if shape(photos, max_gap, &trial) == target {
            trial
        } else {
            kept
        }
    })
}

/// `pins` without the pins of `paths`.
fn unpinned(pins: &Pins, paths: &[PathBuf]) -> Pins {
    let mut without = pins.clone();
    without.apply(&PinChanges {
        set: Vec::new(),
        clear: paths.to_vec(),
    });
    without
}

/// The members of the series of `photos` with `pins`, whatever their kind.
fn shape(photos: &[Photo], max_gap: Duration, pins: &Pins) -> Vec<Vec<usize>> {
    group(photos, max_gap, pins)
        .into_iter()
        .map(|s| s.members)
        .collect()
}

/// The changes that turn `before` into `after`.
fn difference(before: &Pins, after: &Pins) -> PinChanges {
    PinChanges {
        set: after
            .iter()
            .filter(|&(path, pin)| before.get(path) != Some(pin))
            .map(|(path, pin)| (path.to_path_buf(), pin))
            .collect(),
        clear: before
            .iter()
            .filter(|(path, _)| after.get(path).is_none())
            .map(|(path, _)| path.to_path_buf())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::edit::{Edit, plan_edit};
    use super::super::pins::ManualSeriesId;
    use super::super::tests::shots;
    use super::super::{Grouping, SeriesKind};
    use super::*;

    const A: Option<&str> = Some("Canon EOS R6");
    const B: Option<&str> = Some("Pixel 8");
    const GAP: Duration = Duration::from_secs(10);

    fn path(seconds: i64) -> PathBuf {
        PathBuf::from(format!("{seconds}.jpg"))
    }

    fn join(photo: i64, into: i64) -> Edit {
        Edit::Join {
            photo: path(photo),
            into: path(into),
        }
    }

    /// Applies `edits` one after another to photos of camera A taken at
    /// `seconds`, and returns the pinned photos and the series (as seconds,
    /// with whether each is edited) at the end.
    fn after_edits(photos: &[Photo], edits: &[Edit]) -> (Vec<i64>, Vec<(Vec<i64>, bool)>) {
        let mut pins = Pins::default();
        for (n, edit) in edits.iter().enumerate() {
            let next = ManualSeriesId(i64::try_from(n).unwrap() * 10 + 1);
            let changes = plan_edit(photos, GAP, &pins, edit, next).unwrap();
            pins.apply(&changes);
        }
        let seconds = |p: &Photo| -> i64 {
            p.path
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .parse()
                .unwrap()
        };
        let mut pinned: Vec<i64> = pins
            .iter()
            .map(|(p, _)| p.file_stem().unwrap().to_str().unwrap().parse().unwrap())
            .collect();
        pinned.sort_unstable();
        let grouping = Grouping::new(photos, &vec![false; photos.len()], GAP, &pins);
        let series = grouping
            .series
            .iter()
            .map(|s| {
                let members = s.members.iter().map(|&m| seconds(&photos[m])).collect();
                (members, matches!(s.kind, SeriesKind::Manual(_)))
            })
            .collect();
        (pinned, series)
    }

    fn one_camera(seconds: &[i64]) -> Vec<Photo> {
        shots(&seconds.iter().map(|&s| (s, A)).collect::<Vec<_>>())
    }

    #[test]
    fn joining_and_ungrouping_leaves_the_single_photo_unpinned() {
        // 30 is single; 6 and 4 a series.
        let photos = one_camera(&[4, 6, 30]);

        let (pinned, series) =
            after_edits(&photos, &[join(30, 6), Edit::Ungroup { series: path(6) }]);

        assert_eq!(pinned, [4, 6]);
        assert_eq!(series, []);
    }

    #[test]
    fn joining_and_taking_out_again_leaves_nothing_pinned() {
        let photos = one_camera(&[4, 6, 30]);

        let (pinned, series) =
            after_edits(&photos, &[join(30, 6), Edit::Remove { photo: path(30) }]);

        assert_eq!(pinned, Vec::<i64>::new());
        assert_eq!(series, [(vec![6, 4], false)]);
    }

    #[test]
    fn taking_out_and_joining_back_leaves_nothing_pinned() {
        let photos = one_camera(&[0, 2, 4]);

        let (pinned, series) = after_edits(&photos, &[Edit::Remove { photo: path(2) }, join(2, 4)]);

        assert_eq!(pinned, Vec::<i64>::new());
        assert_eq!(series, [(vec![4, 2, 0], false)]);
    }

    #[test]
    fn a_cut_joined_back_photo_by_photo_ends_unpinned() {
        let photos = one_camera(&[0, 2, 4, 6]);
        let cut = Edit::Cut { at: path(2) };

        // Halfway, the parts still differ from the automatic series.
        let (pinned, series) = after_edits(&photos, &[cut.clone(), join(2, 4)]);
        assert_eq!(pinned, [0, 2, 4, 6]);
        assert_eq!(series, [(vec![6, 4, 2], true)]);

        let (pinned, series) = after_edits(&photos, &[cut, join(2, 4), join(0, 2)]);
        assert_eq!(pinned, Vec::<i64>::new());
        assert_eq!(series, [(vec![6, 4, 2, 0], false)]);
    }

    #[test]
    fn a_cut_keeps_both_parts_edited() {
        let photos = one_camera(&[0, 2, 4, 6]);

        let (pinned, series) = after_edits(&photos, &[Edit::Cut { at: path(2) }]);

        assert_eq!(pinned, [0, 2, 4, 6]);
        assert_eq!(series, [(vec![6, 4], true), (vec![2, 0], true)]);
    }

    #[test]
    fn joining_a_single_photo_and_cutting_it_off_again_leaves_nothing_pinned() {
        let photos = one_camera(&[4, 6, 30]);

        let (pinned, series) = after_edits(&photos, &[join(30, 6), Edit::Cut { at: path(6) }]);

        assert_eq!(pinned, Vec::<i64>::new());
        assert_eq!(series, [(vec![6, 4], false)]);
    }

    #[test]
    fn a_photo_moved_to_another_series_and_out_again_keeps_its_pair_broken() {
        // 42 and 40 one series, 6 and 4 another.
        let photos = one_camera(&[4, 6, 40, 42]);

        let (pinned, series) =
            after_edits(&photos, &[join(42, 6), Edit::Remove { photo: path(42) }]);

        // 6 and 4 are automatic again; 42 and 40 stay apart, as the user left them.
        assert_eq!(pinned, [40, 42]);
        assert_eq!(series, [(vec![6, 4], false)]);

        // Restoring either of them brings the pair back and leaves nothing pinned.
        let restore = Edit::Restore { photo: path(40) };
        let (pinned, series) = after_edits(
            &photos,
            &[join(42, 6), Edit::Remove { photo: path(42) }, restore],
        );
        assert_eq!(pinned, Vec::<i64>::new());
        assert_eq!(series, [(vec![42, 40], false), (vec![6, 4], false)]);
    }

    #[test]
    fn ungrouping_a_pair_keeps_both_photos_pinned() {
        let photos = one_camera(&[0, 2]);

        let (pinned, series) = after_edits(&photos, &[Edit::Ungroup { series: path(2) }]);

        assert_eq!(pinned, [0, 2]);
        assert_eq!(series, []);
    }

    #[test]
    fn another_cameras_photo_joined_and_taken_out_leaves_nothing_pinned() {
        // a: 0, 4, 8 one series; b: 6 single, between them in time.
        let photos = shots(&[(0, A), (4, A), (6, B), (8, A)]);

        let (pinned, series) = after_edits(&photos, &[join(6, 8), Edit::Remove { photo: path(6) }]);

        assert_eq!(pinned, Vec::<i64>::new());
        assert_eq!(series, [(vec![8, 4, 0], false)]);
    }

    #[test]
    fn an_edit_next_to_an_earlier_one_leaves_the_earlier_one_alone() {
        // 20, 18 a series, ungrouped; 6, 4 a series with the single 40 joined
        // and taken out.
        let photos = one_camera(&[4, 6, 18, 20, 40]);

        let (pinned, _) = after_edits(
            &photos,
            &[
                Edit::Ungroup { series: path(20) },
                join(40, 6),
                Edit::Remove { photo: path(40) },
            ],
        );

        assert_eq!(pinned, [18, 20]);
    }
}
