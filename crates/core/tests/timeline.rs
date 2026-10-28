// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone};
use sito_core::timeline::{TimeSource, TimelineProgress, build_timeline, read_photo};
use tempfile::TempDir;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/exif")
}

fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(y, mo, d)
        .unwrap()
        .and_hms_opt(h, mi, s)
        .unwrap()
}

fn set_modified(path: &Path, local: NaiveDateTime) {
    let time: SystemTime = Local.from_local_datetime(&local).unwrap().into();
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

/// Copies all EXIF fixtures into a temp folder; the files without a usable
/// EXIF date get known modification times.
fn gallery() -> TempDir {
    let dir = TempDir::new().unwrap();
    for entry in fs::read_dir(fixtures()).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dir.path().join(entry.file_name())).unwrap();
    }
    set_modified(&dir.path().join("no-exif.jpg"), at(2024, 6, 3, 9, 0, 0));
    set_modified(&dir.path().join("broken-date.jpg"), at(2024, 5, 1, 7, 0, 0));
    dir
}

fn names(timeline: &sito_core::timeline::Timeline) -> Vec<String> {
    timeline
        .photos
        .iter()
        .map(|p| p.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn sorts_photos_newest_first() {
    let dir = gallery();

    let timeline = build_timeline(dir.path(), |_| {}).unwrap();

    assert_eq!(
        names(&timeline),
        [
            "no-exif.jpg",                           // 2024-06-03 09:00 (file date)
            "offset_2024-06-02_08-30-00+0200.jpg",   // 2024-06-02 08:30
            "pixel_2024-06-01_12-00-05.jpg",         // 2024-06-01 12:00:05
            "pixel_2024-06-01_12-00-00.jpg",         // 2024-06-01 12:00:00
            "rotated_orientation-6.jpg",             // 2024-05-31 18:15
            "datetime-only_2024-05-30_10-00-00.jpg", // 2024-05-30 10:00
            "broken-date.jpg",                       // 2024-05-01 07:00 (file date)
        ]
    );
}

#[test]
fn falls_back_to_file_date_without_exif_date() {
    let dir = gallery();

    let timeline = build_timeline(dir.path(), |_| {}).unwrap();

    let no_exif = &timeline.photos[0];
    assert_eq!(no_exif.taken_at, at(2024, 6, 3, 9, 0, 0));
    assert_eq!(no_exif.time_source, TimeSource::FileModified);
    assert_eq!(timeline.without_date(), 2);
}

#[test]
fn keeps_exif_details() {
    let dir = gallery();

    let timeline = build_timeline(dir.path(), |_| {}).unwrap();

    let rotated = timeline
        .photos
        .iter()
        .find(|p| p.path.ends_with("rotated_orientation-6.jpg"))
        .unwrap();
    assert_eq!(rotated.taken_at, at(2024, 5, 31, 18, 15, 0));
    assert_eq!(rotated.time_source, TimeSource::Exif);
    assert_eq!(rotated.orientation.value(), 6);
    assert_eq!(rotated.camera.as_deref(), Some("Google Pixel 8"));
    assert_eq!(timeline.root, dir.path());
}

#[test]
fn breaks_ties_by_path() {
    let dir = TempDir::new().unwrap();
    for name in ["b.jpg", "a.jpg", "c.jpg"] {
        fs::copy(
            fixtures().join("pixel_2024-06-01_12-00-00.jpg"),
            dir.path().join(name),
        )
        .unwrap();
    }

    let timeline = build_timeline(dir.path(), |_| {}).unwrap();

    assert_eq!(names(&timeline), ["a.jpg", "b.jpg", "c.jpg"]);
}

#[test]
fn merges_photos_from_nested_folders_by_time() {
    let dir = TempDir::new().unwrap();
    for (fixture, target) in [
        ("datetime-only_2024-05-30_10-00-00.jpg", "root.jpg"),
        ("pixel_2024-06-01_12-00-00.jpg", "DCIM/Camera/2024/06/a.jpg"),
        ("pixel_2024-06-01_12-00-05.jpg", "Pixel/Camera/b.jpg"),
        (
            "offset_2024-06-02_08-30-00+0200.jpg",
            "WhatsApp/Images/c.jpg",
        ),
        ("rotated_orientation-6.jpg", "DCIM/Camera/2024/05/d.jpg"),
    ] {
        let path = dir.path().join(target);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::copy(fixtures().join(fixture), path).unwrap();
    }

    let timeline = build_timeline(dir.path(), |_| {}).unwrap();

    let relative: Vec<PathBuf> = timeline
        .photos
        .iter()
        .map(|p| p.path.strip_prefix(dir.path()).unwrap().to_path_buf())
        .collect();
    assert_eq!(
        relative,
        [
            Path::new("WhatsApp/Images/c.jpg"),     // 2024-06-02 08:30
            Path::new("Pixel/Camera/b.jpg"),        // 2024-06-01 12:00:05
            Path::new("DCIM/Camera/2024/06/a.jpg"), // 2024-06-01 12:00:00
            Path::new("DCIM/Camera/2024/05/d.jpg"), // 2024-05-31 18:15
            Path::new("root.jpg"),                  // 2024-05-30 10:00
        ]
        .map(Path::to_path_buf)
    );
}

#[test]
fn reports_searching_then_reading_progress() {
    let dir = gallery();
    let mut reports = Vec::new();

    build_timeline(dir.path(), |p| reports.push(p)).unwrap();

    let first_reading = reports
        .iter()
        .position(|p| matches!(p, TimelineProgress::Reading { .. }))
        .unwrap();
    assert!(
        reports[..first_reading]
            .iter()
            .all(|p| matches!(p, TimelineProgress::Searching { .. }))
    );
    assert!(first_reading > 0, "searching is reported first");
    assert_eq!(
        reports.last(),
        Some(&TimelineProgress::Reading { done: 7, total: 7 })
    );
}

#[test]
fn handles_thousands_of_photos_quickly() {
    let dir = TempDir::new().unwrap();
    for i in 0..3000 {
        fs::write(dir.path().join(format!("IMG_{i:05}.jpg")), b"").unwrap();
    }
    let mut readings = 0;

    let start = Instant::now();
    let timeline = build_timeline(dir.path(), |p| {
        if matches!(p, TimelineProgress::Reading { .. }) {
            readings += 1;
        }
    })
    .unwrap();

    assert_eq!(timeline.photos.len(), 3000);
    assert!(readings > 3, "reading progress is reported in chunks");
    assert!(
        start.elapsed() < Duration::from_secs(30),
        "took {:?}",
        start.elapsed()
    );
}

#[test]
fn removes_and_reinserts_photos_in_timeline_order() {
    let dir = gallery();
    let mut timeline = build_timeline(dir.path(), |_| {}).unwrap();
    let original = timeline.clone();

    for position in [0, original.photos.len() / 2, original.photos.len() - 1] {
        let path = original.photos[position].path.clone();

        let (removed_at, photo) = timeline.remove(&path).unwrap();
        assert_eq!(removed_at, position);
        assert_eq!(timeline.photos.len(), original.photos.len() - 1);
        assert!(timeline.remove(&path).is_none());

        assert_eq!(timeline.insert(photo), position);
        assert_eq!(timeline, original);
    }
}

#[test]
fn reads_a_single_photo_like_the_timeline_does() {
    let dir = gallery();
    let timeline = build_timeline(dir.path(), |_| {}).unwrap();

    for photo in &timeline.photos {
        assert_eq!(&read_photo(&photo.path).unwrap(), photo);
    }
    assert!(read_photo(&dir.path().join("missing.jpg")).is_err());
}
