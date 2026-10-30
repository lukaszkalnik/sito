//! What a series' header says about its camera.

use chrono::NaiveDateTime;

use super::camera_of;
use crate::timeline::Photo;

/// What a series' header says about its camera.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeriesLabel {
    /// Nothing: no series of another camera overlaps it.
    Plain,
    /// The camera's name, as the series overlaps one of another camera.
    Camera(String),
    /// That the camera is unknown, as the series overlaps one of another
    /// camera.
    UnknownCamera,
}

/// The time span and camera of one series.
struct Span<'a> {
    newest: NaiveDateTime,
    oldest: NaiveDateTime,
    camera: Option<&'a str>,
}

/// The label of each group of `photos` positions (newest first, groups
/// sorted by their newest photo). Groups overlap when their time spans
/// intersect and their cameras differ.
pub(super) fn labels(photos: &[Photo], groups: &[Vec<usize>]) -> Vec<SeriesLabel> {
    let spans: Vec<Option<Span>> = groups.iter().map(|g| span(photos, g)).collect();
    let mut overlapping = vec![false; groups.len()];
    for (i, a) in spans.iter().enumerate() {
        let Some(a) = a else { continue };
        // Later groups start no later, so only those still running when `a`
        // starts can overlap it.
        let later = spans.iter().enumerate().skip(i + 1).map_while(|(j, b)| {
            let b = b.as_ref()?;
            (b.newest >= a.oldest).then_some((j, b))
        });
        for (j, b) in later {
            if a.camera != b.camera {
                overlapping[i] = true;
                overlapping[j] = true;
            }
        }
    }
    spans
        .iter()
        .zip(overlapping)
        .map(|(span, overlaps)| match span {
            Some(span) if overlaps => span.camera.map_or(SeriesLabel::UnknownCamera, |c| {
                SeriesLabel::Camera(c.to_owned())
            }),
            Some(_) | None => SeriesLabel::Plain,
        })
        .collect()
}

fn span<'a>(photos: &'a [Photo], group: &[usize]) -> Option<Span<'a>> {
    let newest = photos.get(*group.first()?)?;
    let oldest = photos.get(*group.last()?)?;
    Some(Span {
        newest: newest.taken_at,
        oldest: oldest.taken_at,
        camera: camera_of(newest),
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::shots;
    use super::super::{DEFAULT_MAX_GAP, find_series};
    use super::*;

    fn found_labels(shots_: &[(i64, Option<&str>)]) -> Vec<SeriesLabel> {
        find_series(&shots(shots_), DEFAULT_MAX_GAP)
            .into_iter()
            .map(|s| s.label)
            .collect()
    }

    #[test]
    fn overlapping_series_name_their_cameras() {
        let a = Some("Canon EOS R6");
        let b = Some("Pixel 8");

        let labels = found_labels(&[(0, a), (2, b), (4, a), (6, b)]);

        assert_eq!(
            labels,
            [
                SeriesLabel::Camera("Pixel 8".to_owned()),
                SeriesLabel::Camera("Canon EOS R6".to_owned()),
            ]
        );
    }

    #[test]
    fn an_unknown_camera_overlapping_is_named_as_unknown() {
        let a = Some("Canon EOS R6");

        let labels = found_labels(&[(0, a), (2, None), (4, a), (6, None)]);

        assert_eq!(
            labels,
            [
                SeriesLabel::UnknownCamera,
                SeriesLabel::Camera("Canon EOS R6".to_owned()),
            ]
        );
    }

    #[test]
    fn series_one_after_another_stay_plain() {
        let a = Some("Canon EOS R6");
        let b = Some("Pixel 8");

        let labels = found_labels(&[(0, a), (2, a), (100, b), (102, b), (200, b), (202, b)]);

        assert_eq!(labels, vec![SeriesLabel::Plain; 3]);
    }

    #[test]
    fn a_series_of_the_same_camera_never_overlaps() {
        let a = Some("Canon EOS R6");

        assert_eq!(found_labels(&[(0, a), (2, a)]), [SeriesLabel::Plain]);
    }

    #[test]
    fn a_short_series_inside_a_long_one_overlaps_only_with_it() {
        let a = Some("Canon EOS R6");
        let b = Some("Pixel 8");

        // a: 0..20 every 4 s; b: 9 and 11; another a series much later.
        let labels = found_labels(&[
            (0, a),
            (4, a),
            (8, a),
            (9, b),
            (11, b),
            (12, a),
            (16, a),
            (20, a),
            (100, a),
            (102, a),
        ]);

        assert_eq!(
            labels,
            [
                SeriesLabel::Plain,
                SeriesLabel::Camera("Canon EOS R6".to_owned()),
                SeriesLabel::Camera("Pixel 8".to_owned()),
            ]
        );
    }
}
