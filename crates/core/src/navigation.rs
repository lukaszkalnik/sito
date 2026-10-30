//! Navigation: stepping through the photos of a view one by one.
//!
//! Positions are display positions of `count` photos (see
//! [`Arrangement`](crate::series::Arrangement)); `series` are their series,
//! sorted by start and not overlapping, as
//! [`Arrangement::ranges`](crate::series::Arrangement::ranges) returns them.

use crate::series::SeriesRange;

/// Which photos are stepped through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {
    /// Only photos in series.
    Series,
    /// Every photo.
    All,
}

/// The photos before and after a position in a view; `None` at the ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Neighbours {
    /// Position of the previous (newer) photo.
    pub previous: Option<usize>,
    /// Position of the next (older) photo.
    pub next: Option<usize>,
}

/// The positions before and after `position` in `view`. `position` itself
/// need not be in the view, e.g. a single photo opened in the series view;
/// then its neighbours are the nearest photos of the view around it.
#[must_use]
pub fn neighbours(count: usize, series: &[SeriesRange], view: View, position: usize) -> Neighbours {
    match view {
        View::All => Neighbours {
            previous: position.checked_sub(1).filter(|&p| p < count),
            next: Some(position + 1).filter(|&p| p < count),
        },
        View::Series => Neighbours {
            previous: previous_in_series(series, position),
            next: next_in_series(series, position),
        },
    }
}

/// The series containing `position`, if any.
#[must_use]
pub fn series_of(series: &[SeriesRange], position: usize) -> Option<SeriesRange> {
    let after = series.partition_point(|s| s.start <= position);
    let candidate = *series.get(after.checked_sub(1)?)?;
    (position < candidate.start + candidate.len).then_some(candidate)
}

/// The photo to show after rejecting the one at `position`: the next photo
/// of its series, else the previous one of its series, so the user can keep
/// sorting the same series. Only a single photo (or the last one left of a
/// series) continues with the next, else the previous, photo of the view.
/// `None` if no photo is left.
#[must_use]
pub fn after_reject(
    count: usize,
    series: &[SeriesRange],
    view: View,
    position: usize,
) -> Option<usize> {
    if let Some(own) = series_of(series, position).filter(|s| s.len > 1) {
        let next = position + 1;
        return Some(if next < own.start + own.len {
            next
        } else {
            position - 1
        });
    }
    let around = neighbours(count, series, view, position);
    around.next.or(around.previous)
}

fn previous_in_series(series: &[SeriesRange], position: usize) -> Option<usize> {
    if let Some(own) = series_of(series, position).filter(|s| position > s.start) {
        return Some(position.min(own.start + own.len) - 1);
    }
    // The last series that ends at or before `position`.
    let ended = series.partition_point(|s| s.start + s.len <= position);
    let last = series.get(ended.checked_sub(1)?)?;
    Some(last.start + last.len - 1)
}

fn next_in_series(series: &[SeriesRange], position: usize) -> Option<usize> {
    if let Some(own) = series_of(series, position).filter(|s| position + 1 < s.start + s.len) {
        return Some(position.max(own.start) + 1);
    }
    // The first series that starts after `position`.
    let started = series.partition_point(|s| s.start <= position);
    series.get(started).map(|s| s.start)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ten photos; series at positions 1–2 and 5–7.
    const SERIES: [SeriesRange; 2] = [
        SeriesRange { start: 1, len: 2 },
        SeriesRange { start: 5, len: 3 },
    ];

    fn around(view: View, position: usize) -> (Option<usize>, Option<usize>) {
        let n = neighbours(10, &SERIES, view, position);
        (n.previous, n.next)
    }

    #[test]
    fn all_view_steps_through_every_photo() {
        assert_eq!(around(View::All, 4), (Some(3), Some(5)));
        assert_eq!(around(View::All, 0), (None, Some(1)));
        assert_eq!(around(View::All, 9), (Some(8), None));
    }

    #[test]
    fn series_view_crosses_from_one_series_into_the_next() {
        assert_eq!(around(View::Series, 2), (Some(1), Some(5)));
        assert_eq!(around(View::Series, 5), (Some(2), Some(6)));
        assert_eq!(around(View::Series, 6), (Some(5), Some(7)));
    }

    #[test]
    fn series_view_stops_at_the_ends() {
        assert_eq!(around(View::Series, 1), (None, Some(2)));
        assert_eq!(around(View::Series, 7), (Some(6), None));
    }

    #[test]
    fn series_view_finds_the_photos_around_a_single_photo() {
        assert_eq!(around(View::Series, 3), (Some(2), Some(5)));
        assert_eq!(around(View::Series, 0), (None, Some(1)));
        assert_eq!(around(View::Series, 9), (Some(7), None));
    }

    #[test]
    fn no_neighbours_without_series_or_photos() {
        assert_eq!(neighbours(10, &[], View::Series, 3), Neighbours::default());
        assert_eq!(neighbours(1, &[], View::All, 0), Neighbours::default());
    }

    #[test]
    fn finds_the_series_of_a_photo() {
        assert_eq!(series_of(&SERIES, 6), Some(SERIES[1]));
        assert_eq!(series_of(&SERIES, 1), Some(SERIES[0]));
        assert_eq!(series_of(&SERIES, 7), Some(SERIES[1]));
    }

    #[test]
    fn single_photos_have_no_series() {
        for position in [0, 3, 4, 8, 9] {
            assert_eq!(series_of(&SERIES, position), None, "position {position}");
        }
        assert_eq!(series_of(&[], 0), None);
    }

    #[test]
    fn after_reject_stays_in_the_series() {
        assert_eq!(after_reject(10, &SERIES, View::Series, 5), Some(6));
        assert_eq!(after_reject(10, &SERIES, View::Series, 7), Some(6));
        assert_eq!(after_reject(10, &SERIES, View::All, 2), Some(1));
    }

    #[test]
    fn after_reject_continues_with_the_view_when_the_series_has_no_other_photo() {
        let shrunk = [
            SeriesRange { start: 1, len: 1 },
            SeriesRange { start: 5, len: 3 },
        ];
        assert_eq!(after_reject(10, &shrunk, View::Series, 1), Some(5));
        assert_eq!(after_reject(10, &shrunk, View::All, 1), Some(2));
    }

    #[test]
    fn after_reject_continues_with_the_next_single_photo_or_the_previous_one() {
        assert_eq!(after_reject(10, &SERIES, View::All, 3), Some(4));
        assert_eq!(after_reject(10, &SERIES, View::All, 9), Some(8));
        assert_eq!(after_reject(1, &[], View::All, 0), None);
    }
}
