// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use sito_core::actions::ActionLog;
use sito_core::moved::MovedPhoto;
use sito_core::series::{
    Arrangement, DEFAULT_MAX_GAP, Edit, Grouping, SeriesKind, SeriesLabel, find_series, plan_edit,
};
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

/// Copies the fixture folder `folder` into a temp dir.
fn copied(folder: &str) -> tempfile::TempDir {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let dir = tempfile::TempDir::new().unwrap();
    for entry in std::fs::read_dir(fixtures.join(folder)).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), dir.path().join(entry.file_name())).unwrap();
    }
    dir
}

/// The series of a fresh scan of `root` with the pins of the log at `db`,
/// as file names, with whether each was edited.
fn rescanned_series(root: &Path, db: &Path, gap: Duration) -> Vec<(Vec<String>, bool)> {
    let timeline = build_timeline(root, |_| {}).unwrap();
    let pins = ActionLog::open(db).unwrap().pins_under(root).unwrap();
    let removed = vec![false; timeline.photos.len()];
    Grouping::new(&timeline.photos, &removed, gap, &pins)
        .series
        .iter()
        .map(|s| {
            let edited = matches!(s.kind, SeriesKind::Manual(_));
            (names(&timeline, &s.members), edited)
        })
        .collect()
}

/// Plans `edit` on a fresh scan of `root` and stores it in the log at `db`.
fn edit(root: &Path, db: &Path, edit: &Edit) {
    let timeline = build_timeline(root, |_| {}).unwrap();
    let log = ActionLog::open(db).unwrap();
    let pins = log.pins_under(root).unwrap();
    let next = log.next_series_id().unwrap();
    let changes = plan_edit(&timeline.photos, DEFAULT_MAX_GAP, &pins, edit, next).unwrap();
    log.apply_pin_changes(&changes).unwrap();
}

/// An automatic series of the given file names.
fn automatic(names: &[&str]) -> (Vec<String>, bool) {
    (names.iter().map(|&n| n.to_owned()).collect(), false)
}

const PIXEL: [&str; 2] = ["pixel_12-00-04.jpg", "pixel_12-00-00.jpg"];

#[test]
fn edits_are_kept_across_rescans_and_gaps_until_restored() {
    let dir = copied("cameras");
    let db_dir = tempfile::TempDir::new().unwrap();
    let db = db_dir.path().join("sito.db");
    let root = dir.path();
    let canon = |name: &str| root.join(format!("canon_{name}.jpg"));

    let photo = canon("12-00-02");
    edit(root, &db, &Edit::Remove { photo });
    for gap in [DEFAULT_MAX_GAP, Duration::from_secs(60)] {
        assert_eq!(rescanned_series(root, &db, gap), [automatic(&PIXEL)]);
    }

    // The photo is renamed outside Sito; the next scan recognises the move.
    let renamed = root.join("renamed.jpg");
    std::fs::rename(canon("12-00-06"), &renamed).unwrap();
    let moved = MovedPhoto {
        from: canon("12-00-06"),
        to: renamed.clone(),
    };
    ActionLog::open(&db).unwrap().move_pins(&[moved]).unwrap();
    let photo = canon("12-00-02");
    edit(root, &db, &Edit::Restore { photo });
    // Only the restored photo is grouped again; the renamed one stays pinned.
    assert_eq!(rescanned_series(root, &db, DEFAULT_MAX_GAP).len(), 1);

    edit(root, &db, &Edit::Restore { photo: renamed });
    assert_eq!(
        rescanned_series(root, &db, DEFAULT_MAX_GAP),
        [
            automatic(&["renamed.jpg", "canon_12-00-02.jpg"]),
            automatic(&PIXEL)
        ]
    );
}
