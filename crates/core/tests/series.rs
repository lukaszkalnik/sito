// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use sito_core::series::{Arrangement, DEFAULT_MAX_GAP, SeriesLabel, find_series};
use sito_core::timeline::{Timeline, build_timeline};

fn timeline(folder: &str) -> Timeline {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    build_timeline(&fixtures.join(folder), |_| {}).unwrap()
}

fn names(timeline: &Timeline, positions: &[usize]) -> Vec<String> {
    positions
        .iter()
        .map(|&p| {
            let path = &timeline.photos[p].path;
            path.file_name().unwrap().to_string_lossy().into_owned()
        })
        .collect()
}

#[test]
fn fixtures_contain_exactly_the_pixel_series() {
    let timeline = timeline("exif");

    let series = find_series(&timeline.photos, DEFAULT_MAX_GAP);

    assert_eq!(series.len(), 1);
    assert_eq!(series[0].label, SeriesLabel::Plain);
    assert_eq!(
        names(&timeline, &series[0].members),
        [
            "pixel_2024-06-01_12-00-05.jpg",
            "pixel_2024-06-01_12-00-00.jpg"
        ]
    );
}

#[test]
fn two_cameras_at_one_moment_form_a_series_each_shown_in_one_piece() {
    let timeline = timeline("cameras");

    let series = find_series(&timeline.photos, DEFAULT_MAX_GAP);
    let arrangement = Arrangement::new(timeline.photos.len(), &series);

    let labels: Vec<_> = series.iter().map(|s| s.label.clone()).collect();
    assert_eq!(
        labels,
        [
            SeriesLabel::Camera("Canon EOS R6".to_owned()),
            SeriesLabel::Camera("Google Pixel 8".to_owned()),
        ]
    );
    assert_eq!(
        names(&timeline, arrangement.order()),
        [
            "canon_12-00-06.jpg",
            "canon_12-00-02.jpg",
            "pixel_12-00-04.jpg",
            "pixel_12-00-00.jpg"
        ]
    );
}
