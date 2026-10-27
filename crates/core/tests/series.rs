// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use sito_core::series::{DEFAULT_MAX_GAP, find_time_series};
use sito_core::timeline::build_timeline;

#[test]
fn fixtures_contain_exactly_the_pixel_series() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/exif");
    let timeline = build_timeline(&fixtures, |_| {}).unwrap();

    let series = find_time_series(&timeline.photos, DEFAULT_MAX_GAP);

    assert_eq!(series.len(), 1);
    let names: Vec<_> = timeline.photos[series[0].start..][..series[0].len]
        .iter()
        .map(|p| p.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        [
            "pixel_2024-06-01_12-00-05.jpg",
            "pixel_2024-06-01_12-00-00.jpg"
        ]
    );
}
