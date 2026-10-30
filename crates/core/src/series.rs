//! Series: photos of one camera shot within seconds of each other.

mod arrangement;
mod label;

use std::collections::HashMap;
use std::time::Duration;

pub use arrangement::{Arrangement, SeriesRange};
pub use label::SeriesLabel;

use crate::timeline::{ExifInfo, Photo};

/// Default largest gap between two neighbouring photos of one series.
pub const DEFAULT_MAX_GAP: Duration = Duration::from_secs(10);

/// Photos of one camera taken in quick succession: at least two when found,
/// fewer once photos of it were rejected (see [`without_removed`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    /// Positions of the photos in the timeline, newest first. Photos of
    /// another camera may lie between them.
    pub members: Vec<usize>,
    /// What the series' header says about its camera.
    pub label: SeriesLabel,
}

/// The series of a timeline and the order its photos are shown in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Grouping {
    /// The series, sorted by their newest photo; members are positions of
    /// the photos left (see [`Grouping::new`]).
    pub series: Vec<Series>,
    /// The photos left in display order, with the series as ranges.
    pub arrangement: Arrangement,
}

impl Grouping {
    /// Finds the series of `all` photos (sorted newest first) and arranges
    /// the photos not marked `true` in `removed` (same length as `all`).
    /// Removed (rejected) photos still shape the series, so a series stays
    /// one while any of its photos is left.
    #[must_use]
    pub fn new(all: &[Photo], removed: &[bool], max_gap: Duration) -> Self {
        let series = without_removed(&find_series(all, max_gap), removed);
        let left = removed.iter().filter(|&&gone| !gone).count();
        Self {
            arrangement: Arrangement::new(left, &series),
            series,
        }
    }
}

/// Finds series in `photos` (sorted newest first, as in
/// [`Timeline::photos`](crate::timeline::Timeline::photos)), sorted by their
/// newest photo.
///
/// Each camera's photos are grouped on their own: two photos of the same
/// camera that follow each other among that camera's photos belong to the
/// same series when both have a capture time and were taken at most
/// `max_gap` apart, even when another camera's photos lie between them.
/// Photos without a known camera (no EXIF camera, or EXIF not read yet) are
/// grouped like one more camera. Photos dated only by the file never join a
/// series: those dates come from copying or downloading and say little about
/// when the photo was taken.
#[must_use]
pub fn find_series(photos: &[Photo], max_gap: Duration) -> Vec<Series> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    // The group each camera's last photo went into.
    let mut open: HashMap<Option<&str>, usize> = HashMap::new();
    for (position, photo) in photos.iter().enumerate() {
        if !photo.time_source.is_capture_time() {
            continue;
        }
        let camera = camera_of(photo);
        let joined = open.get(&camera).copied().filter(|&group| {
            groups[group]
                .last()
                .is_some_and(|&last| belong_together(&photos[last], photo, max_gap))
        });
        if let Some(group) = joined {
            groups[group].push(position);
        } else {
            open.insert(camera, groups.len());
            groups.push(vec![position]);
        }
    }
    let found: Vec<Vec<usize>> = groups.into_iter().filter(|g| g.len() >= 2).collect();
    let labels = label::labels(photos, &found);
    found
        .into_iter()
        .zip(labels)
        .map(|(members, label)| Series { members, label })
        .collect()
}

/// Maps `series` found on a list of photos to the same list without the
/// photos marked `true` in `removed` (same length as that list). A series
/// keeps its shape and label while at least one of its photos is left, so
/// rejecting all but the best shot of a series still shows it as a series.
#[must_use]
pub fn without_removed(series: &[Series], removed: &[bool]) -> Vec<Series> {
    // Position of each photo in the list without the removed ones.
    let mut kept_before = Vec::with_capacity(removed.len());
    let mut kept = 0;
    for &gone in removed {
        kept_before.push((!gone).then_some(kept));
        kept += usize::from(!gone);
    }
    series
        .iter()
        .filter_map(|s| {
            let members: Vec<usize> = s
                .members
                .iter()
                .filter_map(|&m| kept_before.get(m).copied().flatten())
                .collect();
            (!members.is_empty()).then(|| Series {
                members,
                label: s.label.clone(),
            })
        })
        .collect()
}

/// The camera a photo is grouped by; `None` when it is not known.
fn camera_of(photo: &Photo) -> Option<&str> {
    match &photo.exif {
        ExifInfo::Read { camera, .. } => camera.as_deref(),
        ExifInfo::Pending => None,
    }
}

fn belong_together(newer: &Photo, older: &Photo, max_gap: Duration) -> bool {
    if !newer.time_source.is_capture_time() || !older.time_source.is_capture_time() {
        return false;
    }
    (newer.taken_at - older.taken_at)
        .abs()
        .to_std()
        .is_ok_and(|gap| gap <= max_gap)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::PathBuf;

    use chrono::{NaiveDate, NaiveDateTime};

    use super::*;
    use crate::metadata::Orientation;
    use crate::takeout::TakeoutUse;
    use crate::timeline::TimeSource;

    fn at(seconds: i64) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            + chrono::Duration::seconds(seconds)
    }

    /// Photos of the given cameras at the given seconds after noon, newest
    /// first like a timeline.
    pub(crate) fn shots(shots: &[(i64, Option<&str>)]) -> Vec<Photo> {
        let mut photos: Vec<Photo> = shots
            .iter()
            .map(|&(s, camera)| Photo {
                path: PathBuf::from(format!("{s}.jpg")),
                size: 1,
                modified: None,
                taken_at: at(s),
                time_source: TimeSource::Exif,
                sidecar: None,
                companion: None,
                exif: ExifInfo::Read {
                    orientation: Orientation::NORMAL,
                    camera: camera.map(str::to_owned),
                },
            })
            .collect();
        photos.sort_by_key(|p| std::cmp::Reverse(p.taken_at));
        photos
    }

    /// Photos of one camera at the given seconds after noon, newest first.
    fn photos(seconds: &[i64]) -> Vec<Photo> {
        shots(&seconds.iter().map(|&s| (s, None)).collect::<Vec<_>>())
    }

    fn members(series: &[Series]) -> Vec<Vec<usize>> {
        series.iter().map(|s| s.members.clone()).collect()
    }

    fn plain(members: &[usize]) -> Series {
        Series {
            members: members.to_vec(),
            label: SeriesLabel::Plain,
        }
    }

    #[test]
    fn no_series_without_neighbours() {
        assert_eq!(find_series(&[], DEFAULT_MAX_GAP), []);
        assert_eq!(find_series(&photos(&[0]), DEFAULT_MAX_GAP), []);
    }

    #[test]
    fn gap_of_exactly_max_gap_stays_together() {
        let max_gap = Duration::from_secs(10);

        assert_eq!(members(&find_series(&photos(&[0, 10]), max_gap)), [[0, 1]]);
        assert_eq!(find_series(&photos(&[0, 11]), max_gap), []);
    }

    #[test]
    fn chains_neighbours_and_drops_single_photos() {
        // Newest first: 300, 205, 200, 100, 95, 90, 0
        let photos = photos(&[0, 90, 95, 100, 200, 205, 300]);

        let series = find_series(&photos, DEFAULT_MAX_GAP);

        assert_eq!(members(&series), [vec![1, 2], vec![3, 4, 5]]);
    }

    #[test]
    fn interleaved_cameras_form_a_series_each() {
        let a = Some("Canon EOS R6");
        let b = Some("Canon EOS 5D Mark III");
        // Newest first: b@8, a@6, b@4, a@2, a@0
        let photos = shots(&[(0, a), (2, a), (4, b), (6, a), (8, b)]);

        let series = find_series(&photos, DEFAULT_MAX_GAP);

        assert_eq!(members(&series), [vec![0, 2], vec![1, 3, 4]]);
    }

    #[test]
    fn the_gap_is_measured_between_photos_of_one_camera() {
        let a = Some("Pixel 8");
        let b = Some("Canon EOS R6");
        // The other camera's photo in the middle does not bridge 0 and 16.
        let photos = shots(&[(0, a), (8, b), (16, a)]);

        assert_eq!(find_series(&photos, DEFAULT_MAX_GAP), []);
    }

    #[test]
    fn photos_without_camera_and_with_pending_exif_group_together() {
        let mut photos = shots(&[(0, None), (3, Some("Pixel 8")), (6, None)]);
        photos[0].exif = ExifInfo::Pending;

        let series = find_series(&photos, DEFAULT_MAX_GAP);

        // Newest first: pending@6, Pixel@3, none@0.
        assert_eq!(members(&series), [[0, 2]]);
    }

    #[test]
    fn photo_dated_by_file_stays_out_of_a_series_around_it() {
        let mut photos = photos(&[0, 3, 6, 9]);
        photos[1].time_source = TimeSource::FileModified;

        let series = find_series(&photos, DEFAULT_MAX_GAP);

        // Newest first: 9, 6 (file date), 3, 0
        assert_eq!(members(&series), [[0, 2, 3]]);
    }

    #[test]
    fn photos_dated_by_takeout_form_series_with_exif_photos() {
        let mut photos = photos(&[0, 3, 6]);
        photos[0].time_source = TimeSource::Takeout(TakeoutUse::MissingExif);
        photos[2].time_source = TimeSource::Takeout(TakeoutUse::Corrected);

        assert_eq!(members(&find_series(&photos, DEFAULT_MAX_GAP)), [[0, 1, 2]]);
    }

    #[test]
    fn photos_dated_by_file_never_form_a_series() {
        let mut photos = photos(&[0, 1, 2]);
        for photo in &mut photos {
            photo.time_source = TimeSource::FileModified;
        }

        assert_eq!(find_series(&photos, DEFAULT_MAX_GAP), []);
    }

    #[test]
    fn identical_times_belong_together() {
        assert_eq!(
            members(&find_series(&photos(&[5, 5, 5]), DEFAULT_MAX_GAP)),
            [[0, 1, 2]]
        );
    }

    #[test]
    fn a_series_keeps_its_shape_while_one_photo_is_left() {
        // Photos 0–1 and 3–5 form series; 2 and 6 are single.
        let series = [plain(&[0, 1]), plain(&[3, 4, 5])];

        // The second photo of the first series is rejected: one is left.
        let removed = [false, true, false, false, false, false, false];
        assert_eq!(
            members(&without_removed(&series, &removed)),
            [vec![0], vec![2, 3, 4]]
        );

        // Two of the second series and a single are rejected.
        let removed = [false, false, true, true, false, true, false];
        assert_eq!(
            members(&without_removed(&series, &removed)),
            [vec![0, 1], vec![2]]
        );
    }

    #[test]
    fn interleaved_series_keep_their_photos_after_a_reject() {
        let series = [plain(&[0, 2]), plain(&[1, 3, 4])];

        let removed = [false, true, false, false, false];

        assert_eq!(
            members(&without_removed(&series, &removed)),
            [vec![0, 1], vec![2, 3]]
        );
    }

    #[test]
    fn a_series_keeps_its_label_after_a_reject() {
        let series = [Series {
            members: vec![0, 1],
            label: SeriesLabel::Camera("Pixel 8".to_owned()),
        }];

        let kept = without_removed(&series, &[true, false]);

        assert_eq!(kept[0].label, SeriesLabel::Camera("Pixel 8".to_owned()));
    }

    #[test]
    fn a_series_with_all_photos_rejected_is_gone() {
        let series = [plain(&[0, 1]), plain(&[2, 3])];

        let removed = [true, true, false, false];

        assert_eq!(members(&without_removed(&series, &removed)), [[0, 1]]);
    }

    #[test]
    fn a_grouping_arranges_the_photos_left() {
        let a = Some("Canon EOS R6");
        let b = Some("Pixel 8");
        // Newest first: b@6, a@4, b@2, a@0; a@4 is rejected.
        let all = shots(&[(0, a), (2, b), (4, a), (6, b)]);

        let grouping = Grouping::new(&all, &[false, true, false, false], DEFAULT_MAX_GAP);

        // Left: b@6, b@2, a@0; a's series keeps its one photo.
        assert_eq!(members(&grouping.series), [vec![0, 1], vec![2]]);
        assert_eq!(grouping.arrangement.order(), [0, 1, 2]);
    }

    #[test]
    fn nothing_removed_leaves_the_series_unchanged() {
        let series = [plain(&[1, 2, 3])];

        assert_eq!(without_removed(&series, &[false; 5]), series);
    }
}
