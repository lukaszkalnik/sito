//! The order in which photos are shown: by time, each series in one piece.

use super::Series;

/// A series as a run of consecutive positions of an [`Arrangement`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeriesRange {
    /// Display position of the series' first (newest) photo.
    pub start: usize,
    /// Number of photos in the series.
    pub len: usize,
}

/// The photos of a timeline in the order they are shown: newest first, with
/// each series in one piece at the place of its newest photo. Photos outside
/// series keep their place in time; photos of another camera that were taken
/// during a series come after it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Arrangement {
    /// Timeline position of the photo at each display position.
    order: Vec<usize>,
    /// Display position of the photo at each timeline position.
    display: Vec<usize>,
    /// The series, sorted by start.
    ranges: Vec<SeriesRange>,
}

impl Arrangement {
    /// Arranges a timeline of `count` photos with `series` (sorted by their
    /// newest photo, as [`find_series`](super::find_series) and
    /// [`without_removed`](super::without_removed) return them). Members at
    /// or past `count` are ignored.
    #[must_use]
    pub fn new(count: usize, series: &[Series]) -> Self {
        let mut first_of: Vec<Option<&Series>> = vec![None; count];
        let mut in_series = vec![false; count];
        for s in series {
            // Members are newest first, so the others lie past the first.
            let Some(slot) = s.members.first().and_then(|&m| first_of.get_mut(m)) else {
                continue;
            };
            *slot = Some(s);
            for &m in &s.members {
                if let Some(flag) = in_series.get_mut(m) {
                    *flag = true;
                }
            }
        }
        let mut order = Vec::with_capacity(count);
        let mut ranges = Vec::with_capacity(series.len());
        for position in 0..count {
            if let Some(s) = first_of[position] {
                let start = order.len();
                order.extend(s.members.iter().copied().filter(|&m| m < count));
                ranges.push(SeriesRange {
                    start,
                    len: order.len() - start,
                });
            } else if !in_series[position] {
                order.push(position);
            }
        }
        let mut display = vec![0; count];
        for (shown, &position) in order.iter().enumerate() {
            display[position] = shown;
        }
        Self {
            order,
            display,
            ranges,
        }
    }

    /// Timeline positions in display order.
    #[must_use]
    pub fn order(&self) -> &[usize] {
        &self.order
    }

    /// The series as ranges of display positions, sorted by start and not
    /// overlapping.
    #[must_use]
    pub fn ranges(&self) -> &[SeriesRange] {
        &self.ranges
    }

    /// The display position of the photo at timeline position `position`.
    #[must_use]
    pub fn display_of(&self, position: usize) -> Option<usize> {
        self.display.get(position).copied()
    }

    /// The timeline position of the photo shown at `shown`.
    #[must_use]
    pub fn timeline_of(&self, shown: usize) -> Option<usize> {
        self.order.get(shown).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::super::SeriesLabel;
    use super::*;

    fn series(members: &[usize]) -> Series {
        Series {
            members: members.to_vec(),
            label: SeriesLabel::Plain,
        }
    }

    #[test]
    fn without_series_the_order_is_the_timeline() {
        let arrangement = Arrangement::new(3, &[]);

        assert_eq!(arrangement.order(), [0, 1, 2]);
        assert_eq!(arrangement.ranges(), []);
    }

    #[test]
    fn contiguous_series_keep_their_place() {
        let arrangement = Arrangement::new(5, &[series(&[1, 2]), series(&[3, 4])]);

        assert_eq!(arrangement.order(), [0, 1, 2, 3, 4]);
        assert_eq!(
            arrangement.ranges(),
            [
                SeriesRange { start: 1, len: 2 },
                SeriesRange { start: 3, len: 2 }
            ]
        );
    }

    #[test]
    fn interleaved_series_are_each_shown_in_one_piece_at_their_newest_photo() {
        // 0 and 2 are one camera, 1, 3 and 5 another; 4 is a single photo.
        let arrangement = Arrangement::new(7, &[series(&[0, 2]), series(&[1, 3, 5])]);

        assert_eq!(arrangement.order(), [0, 2, 1, 3, 5, 4, 6]);
        assert_eq!(
            arrangement.ranges(),
            [
                SeriesRange { start: 0, len: 2 },
                SeriesRange { start: 2, len: 3 }
            ]
        );
    }

    #[test]
    fn maps_between_timeline_and_display_positions() {
        let arrangement = Arrangement::new(4, &[series(&[0, 2])]);

        assert_eq!(arrangement.order(), [0, 2, 1, 3]);
        assert_eq!(arrangement.display_of(1), Some(2));
        assert_eq!(arrangement.display_of(2), Some(1));
        assert_eq!(arrangement.timeline_of(1), Some(2));
        assert_eq!(arrangement.display_of(4), None);
        assert_eq!(arrangement.timeline_of(4), None);
    }

    #[test]
    fn members_past_the_timeline_are_ignored() {
        let arrangement = Arrangement::new(2, &[series(&[0, 5])]);

        assert_eq!(arrangement.order(), [0, 1]);
        assert_eq!(arrangement.ranges(), [SeriesRange { start: 0, len: 1 }]);
    }
}
