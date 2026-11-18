//! Editing series by hand: what each edit does to the pins.

use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;

use super::needless::without_needless_pins;
use super::pins::{ManualSeriesId, Pin, PinChanges, Pins};
use super::restore::restored_paths;
use super::{Series, SeriesKind, group};
use crate::timeline::Photo;

/// An edit of the series, naming photos by path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Edit {
    /// Splits the photo's series before it: the photos before it stay one
    /// series, it and the photos after it form another.
    Cut {
        /// A photo of a series other than its first.
        at: PathBuf,
    },
    /// Takes the photo out of its series, as a single photo; the rest stays
    /// one series.
    Remove {
        /// A photo of a series.
        photo: PathBuf,
    },
    /// Puts the photo into the series of `into`; a series it came from
    /// keeps the rest.
    Join {
        /// The photo to move.
        photo: PathBuf,
        /// A photo of the series to join, not the photo's own.
        into: PathBuf,
    },
    /// Makes every photo of the series a single photo.
    Ungroup {
        /// A photo of the series.
        series: PathBuf,
    },
    /// Unpins the stretch of edited photos around the photo (see
    /// [`plan_edit`]), so they are grouped automatically again.
    Restore {
        /// A photo of a manual series, or a photo pinned on its own.
        photo: PathBuf,
    },
}

/// Why an edit cannot be made on the current series.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EditError {
    /// A photo of the edit is not on the timeline.
    #[error("series edit: {0} is not on the timeline")]
    UnknownPhoto(PathBuf),
    /// The edit needs a photo of a series, but this one is not in any.
    #[error("series edit: {0} is not in a series")]
    NotInSeries(PathBuf),
    /// A series cannot be split before its first photo.
    #[error("series edit: {0} is the first photo of its series")]
    FirstOfSeries(PathBuf),
    /// The photo already is in the series it should join.
    #[error("series edit: {0} already is in that series")]
    SameSeries(PathBuf),
    /// There is no edit to take back: the photo is grouped automatically.
    #[error("series edit: {0} is not pinned")]
    NotPinned(PathBuf),
}

/// Works out how `edit` changes the pins, on the series of `photos` (the
/// timeline with its rejected photos, sorted newest first) for `max_gap`
/// with the current `pins`. New manual series get ids from `next_free` on.
///
/// „Restore” unpins the run of pinned photos around the photo in its
/// camera's chain, with every photo of a manual series among them, so a
/// photo taken out of a series comes back together with the rest of it.
///
/// A part of a series left with one photo (counting rejected ones, which
/// still belong to it) is pinned as a single photo; a part with more becomes
/// a manual series. Every photo of an edited series is pinned, so it stays
/// as the user left it, unless the edit brings its stretch back to how
/// automatic grouping has it (see [`without_needless_pins`]).
///
/// # Errors
///
/// Returns an [`EditError`] when the edit does not fit the current series,
/// e.g. because they changed in another window meanwhile.
pub fn plan_edit(
    photos: &[Photo],
    max_gap: Duration,
    pins: &Pins,
    edit: &Edit,
    next_free: ManualSeriesId,
) -> Result<PinChanges, EditError> {
    let series = group(photos, max_gap, pins);
    let current = Current { photos, series };
    let mut ids = FreshIds(next_free.0);
    let set = match edit {
        Edit::Cut { at } => current.cut(at, &mut ids)?,
        Edit::Remove { photo } => current.remove(photo, &mut ids)?,
        Edit::Join { photo, into } => current.join(photo, into, &mut ids)?,
        Edit::Ungroup { series } => current.ungroup(series)?,
        Edit::Restore { photo } => {
            return Ok(PinChanges {
                set: Vec::new(),
                clear: current.restore(photo, pins)?,
            });
        }
    };
    let changes = PinChanges {
        set: set
            .into_iter()
            .map(|(position, pin)| (current.photos[position].path.clone(), pin))
            .collect(),
        clear: Vec::new(),
    };
    Ok(without_needless_pins(photos, max_gap, pins, &changes))
}

/// Hands out ids for new manual series.
struct FreshIds(i64);

impl FreshIds {
    fn take(&mut self) -> ManualSeriesId {
        self.0 += 1;
        ManualSeriesId(self.0 - 1)
    }
}

/// New pins, by position in the timeline.
type NewPins = Vec<(usize, Pin)>;

/// The timeline and its series as they are before an edit.
struct Current<'a> {
    photos: &'a [Photo],
    series: Vec<Series>,
}

impl Current<'_> {
    fn position_of(&self, path: &Path) -> Result<usize, EditError> {
        self.photos
            .iter()
            .position(|photo| photo.path == path)
            .ok_or_else(|| EditError::UnknownPhoto(path.to_path_buf()))
    }

    /// The series of the photo at `position`, if it is in one.
    fn series_of(&self, position: usize) -> Option<&Series> {
        self.series.iter().find(|s| s.members.contains(&position))
    }

    /// The photo's position and its series.
    fn in_series(&self, path: &Path) -> Result<(usize, &Series), EditError> {
        let position = self.position_of(path)?;
        let series = self
            .series_of(position)
            .ok_or_else(|| EditError::NotInSeries(path.to_path_buf()))?;
        Ok((position, series))
    }

    fn cut(&self, at: &Path, ids: &mut FreshIds) -> Result<NewPins, EditError> {
        let (position, series) = self.in_series(at)?;
        let index = series
            .members
            .iter()
            .position(|&m| m == position)
            .filter(|&index| index > 0)
            .ok_or_else(|| EditError::FirstOfSeries(at.to_path_buf()))?;
        let (before, after) = series.members.split_at(index);
        let mut pins = part(before, kept_id(series, ids));
        pins.extend(part(after, ids.take()));
        Ok(pins)
    }

    fn remove(&self, photo: &Path, ids: &mut FreshIds) -> Result<NewPins, EditError> {
        let (position, series) = self.in_series(photo)?;
        let mut pins = vec![(position, Pin::Single)];
        pins.extend(rest_of(series, position, ids));
        Ok(pins)
    }

    fn join(&self, photo: &Path, into: &Path, ids: &mut FreshIds) -> Result<NewPins, EditError> {
        let position = self.position_of(photo)?;
        let (_, target) = self.in_series(into)?;
        if target.members.contains(&position) {
            return Err(EditError::SameSeries(photo.to_path_buf()));
        }
        let id = kept_id(target, ids);
        let mut pins: NewPins = target
            .members
            .iter()
            .chain([&position])
            .map(|&m| (m, Pin::Series(id)))
            .collect();
        if let Some(source) = self.series_of(position) {
            pins.extend(rest_of(source, position, ids));
        }
        Ok(pins)
    }

    fn ungroup(&self, photo: &Path) -> Result<NewPins, EditError> {
        let (_, series) = self.in_series(photo)?;
        Ok(series.members.iter().map(|&m| (m, Pin::Single)).collect())
    }

    /// The paths to unpin (see [`restored_paths`]).
    fn restore(&self, photo: &Path, pins: &Pins) -> Result<Vec<PathBuf>, EditError> {
        let position = self.position_of(photo)?;
        restored_paths(self.photos, pins, position)
            .ok_or_else(|| EditError::NotPinned(photo.to_path_buf()))
    }
}

/// The id a series keeps when it is edited: its own if it is manual, a new
/// one if it was found automatically.
fn kept_id(series: &Series, ids: &mut FreshIds) -> ManualSeriesId {
    match series.kind {
        SeriesKind::Manual(id) => id,
        SeriesKind::Automatic => ids.take(),
    }
}

/// The pins of the rest of `series` once the photo at `position` left it.
fn rest_of(series: &Series, position: usize, ids: &mut FreshIds) -> NewPins {
    let rest: Vec<usize> = series
        .members
        .iter()
        .copied()
        .filter(|&m| m != position)
        .collect();
    part(&rest, kept_id(series, ids))
}

/// The pins of one part of an edited series: manual series `id`, or a single
/// photo when only one is left.
fn part(members: &[usize], id: ManualSeriesId) -> NewPins {
    let pin = if members.len() >= 2 {
        Pin::Series(id)
    } else {
        Pin::Single
    };
    members.iter().map(|&m| (m, pin)).collect()
}

#[cfg(test)]
mod tests {
    use super::super::tests::shots;
    use super::super::{DEFAULT_MAX_GAP, Grouping};
    use super::*;

    const A: Option<&str> = Some("Canon EOS R6");
    const B: Option<&str> = Some("Pixel 8");
    const NEXT: ManualSeriesId = ManualSeriesId(10);

    fn path(seconds: i64) -> PathBuf {
        PathBuf::from(format!("{seconds}.jpg"))
    }

    /// Photos of one camera at the given seconds after noon, newest first.
    fn photos(seconds: &[i64]) -> Vec<Photo> {
        shots(&seconds.iter().map(|&s| (s, A)).collect::<Vec<_>>())
    }

    /// Plans `edit` and applies it to `pins`, returning the series after it
    /// as the seconds of their photos, with whether each is manual.
    fn edited(photos: &[Photo], pins: &mut Pins, edit: &Edit, gap: u64) -> Vec<(Vec<i64>, bool)> {
        let gap = Duration::from_secs(gap);
        let changes = plan_edit(photos, gap, pins, edit, NEXT).unwrap();
        pins.apply(&changes);
        series_of(photos, pins, gap)
    }

    fn series_of(photos: &[Photo], pins: &Pins, gap: Duration) -> Vec<(Vec<i64>, bool)> {
        let grouping = Grouping::new(photos, &vec![false; photos.len()], gap, pins);
        grouping
            .series
            .iter()
            .map(|s| {
                let seconds = s.members.iter().map(|&m| seconds_of(&photos[m])).collect();
                (seconds, matches!(s.kind, SeriesKind::Manual(_)))
            })
            .collect()
    }

    fn seconds_of(photo: &Photo) -> i64 {
        photo
            .path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| stem.parse().ok())
            .unwrap()
    }

    fn pin_of(pins: &Pins, seconds: i64) -> Option<Pin> {
        pins.get(&path(seconds))
    }

    #[test]
    fn cutting_splits_a_series_into_two_manual_ones() {
        let photos = photos(&[0, 2, 4, 6, 8]);
        let mut pins = Pins::default();

        let series = edited(&photos, &mut pins, &Edit::Cut { at: path(4) }, 10);

        assert_eq!(series, [(vec![8, 6], true), (vec![4, 2, 0], true)]);
        assert_eq!(pin_of(&pins, 8), Some(Pin::Series(ManualSeriesId(10))));
        assert_eq!(pin_of(&pins, 4), Some(Pin::Series(ManualSeriesId(11))));
    }

    #[test]
    fn cutting_leaves_a_part_of_one_photo_single() {
        let photos = photos(&[0, 2, 4]);
        let mut pins = Pins::default();

        let series = edited(&photos, &mut pins, &Edit::Cut { at: path(2) }, 10);

        assert_eq!(series, [(vec![2, 0], true)]);
        assert_eq!(pin_of(&pins, 4), Some(Pin::Single));
    }

    #[test]
    fn a_manual_series_keeps_its_id_for_the_part_before_the_cut() {
        let photos = photos(&[0, 2, 4, 6]);
        let id = ManualSeriesId(3);
        let mut pins: Pins = [0, 2, 4, 6]
            .into_iter()
            .map(|s| (path(s), Pin::Series(id)))
            .collect();

        edited(&photos, &mut pins, &Edit::Cut { at: path(2) }, 10);

        assert_eq!(pin_of(&pins, 6), Some(Pin::Series(id)));
        assert_eq!(pin_of(&pins, 2), Some(Pin::Series(NEXT)));
    }

    #[test]
    fn a_series_cannot_be_cut_before_its_first_photo() {
        let photos = photos(&[0, 2]);

        let result = plan_edit(
            &photos,
            DEFAULT_MAX_GAP,
            &Pins::default(),
            &Edit::Cut { at: path(2) },
            NEXT,
        );

        assert_eq!(result, Err(EditError::FirstOfSeries(path(2))));
    }

    #[test]
    fn removing_makes_the_photo_single_and_keeps_the_rest_together() {
        let photos = photos(&[0, 2, 4]);
        let mut pins = Pins::default();

        // Without pinning the rest, 4 and 0 would be split by the pinned 2.
        let series = edited(&photos, &mut pins, &Edit::Remove { photo: path(2) }, 3);

        assert_eq!(series, [(vec![4, 0], true)]);
        assert_eq!(pin_of(&pins, 2), Some(Pin::Single));
    }

    #[test]
    fn removing_from_a_pair_leaves_two_single_photos() {
        let photos = photos(&[0, 2]);
        let mut pins = Pins::default();

        let series = edited(&photos, &mut pins, &Edit::Remove { photo: path(0) }, 10);

        assert_eq!(series, []);
        assert_eq!(pin_of(&pins, 2), Some(Pin::Single));
    }

    #[test]
    fn a_single_photo_joins_the_series_next_to_it() {
        // 30 is too far from the series 6, 4.
        let photos = photos(&[4, 6, 30]);
        let mut pins = Pins::default();

        let edit = Edit::Join {
            photo: path(30),
            into: path(6),
        };
        let series = edited(&photos, &mut pins, &edit, 10);

        assert_eq!(series, [(vec![30, 6, 4], true)]);
    }

    #[test]
    fn a_photo_joining_another_series_leaves_the_rest_of_its_own() {
        let photos = photos(&[0, 2, 4, 30, 32]);
        let mut pins = Pins::default();

        let edit = Edit::Join {
            photo: path(4),
            into: path(30),
        };
        let series = edited(&photos, &mut pins, &edit, 10);

        assert_eq!(series, [(vec![32, 30, 4], true), (vec![2, 0], true)]);
    }

    #[test]
    fn a_photo_cannot_join_its_own_series() {
        let photos = photos(&[0, 2]);
        let edit = Edit::Join {
            photo: path(0),
            into: path(2),
        };

        let result = plan_edit(&photos, DEFAULT_MAX_GAP, &Pins::default(), &edit, NEXT);

        assert_eq!(result, Err(EditError::SameSeries(path(0))));
    }

    #[test]
    fn joining_needs_a_series_to_join() {
        let photos = photos(&[0, 30]);
        let edit = Edit::Join {
            photo: path(0),
            into: path(30),
        };

        let result = plan_edit(&photos, DEFAULT_MAX_GAP, &Pins::default(), &edit, NEXT);

        assert_eq!(result, Err(EditError::NotInSeries(path(30))));
    }

    #[test]
    fn ungrouping_pins_every_photo_on_its_own() {
        let photos = photos(&[0, 2, 4]);
        let mut pins = Pins::default();

        let series = edited(&photos, &mut pins, &Edit::Ungroup { series: path(2) }, 60);

        assert_eq!(series, []);
        assert!(
            [0, 2, 4]
                .iter()
                .all(|&s| pin_of(&pins, s) == Some(Pin::Single))
        );
    }

    #[test]
    fn restoring_a_cut_series_regroups_both_parts_at_the_current_gap() {
        // Cut at 10 s, then the gap is raised to 60 s and the cut restored.
        let photos = photos(&[0, 2, 4, 6, 40]);
        let mut pins = Pins::default();
        edited(&photos, &mut pins, &Edit::Cut { at: path(2) }, 10);

        let series = edited(&photos, &mut pins, &Edit::Restore { photo: path(6) }, 60);

        assert_eq!(series, [(vec![40, 6, 4, 2, 0], false)]);
        assert!(pins.is_empty());
    }

    #[test]
    fn restoring_a_photo_taken_out_puts_it_back_into_its_series() {
        let photos = photos(&[0, 2, 4]);
        let mut pins = Pins::default();
        edited(&photos, &mut pins, &Edit::Remove { photo: path(2) }, 10);

        let series = edited(&photos, &mut pins, &Edit::Restore { photo: path(2) }, 10);

        assert_eq!(series, [(vec![4, 2, 0], false)]);
    }

    #[test]
    fn restoring_also_unpins_photos_of_the_series_that_are_gone() {
        let photos = photos(&[0, 2]);
        let id = ManualSeriesId(1);
        let mut pins: Pins = [0, 2, 99]
            .into_iter()
            .map(|s| (path(s), Pin::Series(id)))
            .collect();

        edited(&photos, &mut pins, &Edit::Restore { photo: path(2) }, 10);

        assert!(pins.is_empty());
    }

    #[test]
    fn restoring_a_pinned_single_photo_lets_the_series_around_it_join() {
        let photos = photos(&[0, 2, 4]);
        let mut pins: Pins = [(path(2), Pin::Single)].into_iter().collect();
        assert_eq!(series_of(&photos, &pins, DEFAULT_MAX_GAP), []);

        let series = edited(&photos, &mut pins, &Edit::Restore { photo: path(2) }, 10);

        assert_eq!(series, [(vec![4, 2, 0], false)]);
    }

    #[test]
    fn an_automatic_series_or_photo_has_nothing_to_restore() {
        let photos = photos(&[0, 2, 30]);
        let restore = |s| {
            plan_edit(
                &photos,
                DEFAULT_MAX_GAP,
                &Pins::default(),
                &Edit::Restore { photo: path(s) },
                NEXT,
            )
        };

        assert_eq!(restore(2), Err(EditError::NotPinned(path(2))));
        assert_eq!(restore(30), Err(EditError::NotPinned(path(30))));
    }

    #[test]
    fn an_edit_of_a_photo_not_on_the_timeline_is_refused() {
        let photos = photos(&[0, 2]);

        let result = plan_edit(
            &photos,
            DEFAULT_MAX_GAP,
            &Pins::default(),
            &Edit::Remove { photo: path(5) },
            NEXT,
        );

        assert_eq!(result, Err(EditError::UnknownPhoto(path(5))));
    }

    #[test]
    fn joining_a_photo_and_taking_it_out_again_leaves_nothing_edited() {
        // 30 is a single photo, 6 and 4 a series.
        let photos = photos(&[4, 6, 30]);
        let mut pins = Pins::default();
        edited(
            &photos,
            &mut pins,
            &Edit::Join {
                photo: path(30),
                into: path(6),
            },
            10,
        );
        assert!(!pins.is_empty());

        let series = edited(&photos, &mut pins, &Edit::Remove { photo: path(30) }, 10);

        assert_eq!(series, [(vec![6, 4], false)]);
        assert!(pins.is_empty());
    }

    #[test]
    fn an_edit_that_differs_from_automatic_grouping_keeps_its_pins() {
        let photos = photos(&[0, 2, 4, 6]);
        let mut pins = Pins::default();

        let series = edited(&photos, &mut pins, &Edit::Cut { at: path(2) }, 10);

        // Unpinned, the parts would join again, so both stay edited.
        assert_eq!(series, [(vec![6, 4], true), (vec![2, 0], true)]);
    }

    #[test]
    fn an_edited_series_stays_with_any_gap() {
        let photos = photos(&[0, 2, 4, 6, 8, 20]);
        let mut pins = Pins::default();
        edited(&photos, &mut pins, &Edit::Cut { at: path(4) }, 10);

        for gap in [1, 10, 60] {
            assert_eq!(
                series_of(&photos, &pins, Duration::from_secs(gap)),
                [(vec![8, 6], true), (vec![4, 2, 0], true)],
                "gap {gap}"
            );
        }
    }

    #[test]
    fn edits_work_on_interleaved_cameras() {
        // a: 0, 4, 8; b: 2, 6. Newest first: a@8, b@6, a@4, b@2, a@0.
        let photos = shots(&[(0, A), (2, B), (4, A), (6, B), (8, A)]);
        let mut pins = Pins::default();

        let edit = Edit::Join {
            photo: path(2),
            into: path(4),
        };
        let series = edited(&photos, &mut pins, &edit, 10);

        assert_eq!(series, [(vec![8, 4, 2, 0], true)]);
        // b@6 is left alone, so it is pinned as a single photo.
        assert_eq!(pin_of(&pins, 6), Some(Pin::Single));
    }

    #[test]
    fn a_rejected_photo_counts_for_the_part_it_is_in() {
        // 4 is rejected; cutting before 2 leaves 6 with it.
        let photos = photos(&[0, 2, 4, 6]);
        let mut pins = Pins::default();
        let changes = plan_edit(
            &photos,
            DEFAULT_MAX_GAP,
            &pins,
            &Edit::Cut { at: path(2) },
            NEXT,
        )
        .unwrap();
        pins.apply(&changes);

        let grouping = Grouping::new(
            &photos,
            &[false, true, false, false],
            DEFAULT_MAX_GAP,
            &pins,
        );

        // Left: 6, 2, 0; 6 is a series of one, 2 and 0 a series.
        let lens: Vec<usize> = grouping.series.iter().map(|s| s.members.len()).collect();
        assert_eq!(lens, [1, 2]);
    }
}
