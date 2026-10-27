//! Series: photos shot within seconds of each other.

use std::time::Duration;

use crate::timeline::{Photo, TimeSource};

/// Default largest gap between two neighbouring photos of one series.
pub const DEFAULT_MAX_GAP: Duration = Duration::from_secs(10);

/// A run of at least two consecutive photos of a timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Series {
    /// Index of the series' first (newest) photo in the timeline.
    pub start: usize,
    /// Number of photos in the series (at least 2).
    pub len: usize,
}

/// Finds series in `photos` (sorted newest first, as in
/// [`Timeline::photos`](crate::timeline::Timeline::photos)).
///
/// Two neighbours belong to the same series when both have an EXIF capture
/// time and were taken at most `max_gap` apart. Photos dated only by the file
/// never join a series: those dates come from copying or downloading and say
/// little about when the photo was taken.
#[must_use]
pub fn find_time_series(photos: &[Photo], max_gap: Duration) -> Vec<Series> {
    let mut series = Vec::new();
    let mut start = 0;
    for i in 1..=photos.len() {
        let continues = i < photos.len() && belong_together(&photos[i - 1], &photos[i], max_gap);
        if !continues {
            if i - start >= 2 {
                series.push(Series {
                    start,
                    len: i - start,
                });
            }
            start = i;
        }
    }
    series
}

fn belong_together(newer: &Photo, older: &Photo, max_gap: Duration) -> bool {
    if newer.time_source != TimeSource::Exif || older.time_source != TimeSource::Exif {
        return false;
    }
    (newer.taken_at - older.taken_at)
        .abs()
        .to_std()
        .is_ok_and(|gap| gap <= max_gap)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::{NaiveDate, NaiveDateTime};

    use super::*;
    use crate::metadata::Orientation;

    fn at(seconds: i64) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            + chrono::Duration::seconds(seconds)
    }

    /// Photos at the given seconds after noon, newest first like a timeline.
    fn photos(seconds: &[i64]) -> Vec<Photo> {
        let mut photos: Vec<Photo> = seconds
            .iter()
            .map(|&s| Photo {
                path: PathBuf::from(format!("{s}.jpg")),
                size: 1,
                modified: None,
                taken_at: at(s),
                time_source: TimeSource::Exif,
                orientation: Orientation::NORMAL,
                camera: None,
            })
            .collect();
        photos.sort_by_key(|p| std::cmp::Reverse(p.taken_at));
        photos
    }

    fn ranges(series: &[Series]) -> Vec<(usize, usize)> {
        series.iter().map(|s| (s.start, s.len)).collect()
    }

    #[test]
    fn no_series_without_neighbours() {
        assert_eq!(find_time_series(&[], DEFAULT_MAX_GAP), []);
        assert_eq!(find_time_series(&photos(&[0]), DEFAULT_MAX_GAP), []);
    }

    #[test]
    fn gap_of_exactly_max_gap_stays_together() {
        let max_gap = Duration::from_secs(10);

        assert_eq!(
            ranges(&find_time_series(&photos(&[0, 10]), max_gap)),
            [(0, 2)]
        );
        assert_eq!(find_time_series(&photos(&[0, 11]), max_gap), []);
    }

    #[test]
    fn chains_neighbours_and_drops_single_photos() {
        // Newest first: 300, 205, 200, 100, 95, 90, 0
        let photos = photos(&[0, 90, 95, 100, 200, 205, 300]);

        let series = find_time_series(&photos, DEFAULT_MAX_GAP);

        assert_eq!(ranges(&series), [(1, 2), (3, 3)]);
    }

    #[test]
    fn photo_dated_by_file_breaks_a_series() {
        let mut photos = photos(&[0, 3, 6, 9]);
        photos[1].time_source = TimeSource::FileModified;

        let series = find_time_series(&photos, DEFAULT_MAX_GAP);

        // Newest first: 9, 6 (file date), 3, 0
        assert_eq!(ranges(&series), [(2, 2)]);
    }

    #[test]
    fn photos_dated_by_file_never_form_a_series() {
        let mut photos = photos(&[0, 1, 2]);
        for photo in &mut photos {
            photo.time_source = TimeSource::FileModified;
        }

        assert_eq!(find_time_series(&photos, DEFAULT_MAX_GAP), []);
    }

    #[test]
    fn identical_times_belong_together() {
        assert_eq!(
            ranges(&find_time_series(&photos(&[5, 5, 5]), DEFAULT_MAX_GAP)),
            [(0, 3)]
        );
    }
}
