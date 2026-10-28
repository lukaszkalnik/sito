// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use sito_core::reject::{reject, restore};
use sito_core::takeout::TakeoutUse;
use sito_core::timeline::{Photo, TimeSource, build_timeline, read_photo};
use tempfile::TempDir;

/// A copy of the Takeout album fixture (see fixtures/README.md).
fn album() -> TempDir {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/takeout/Album");
    let dir = TempDir::new().unwrap();
    for entry in fs::read_dir(fixtures).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dir.path().join(entry.file_name())).unwrap();
    }
    dir
}

fn at(text: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").unwrap()
}

/// A UTC time as wall-clock time on this computer, like a Takeout date is shown.
fn local(utc: &str) -> NaiveDateTime {
    Utc.from_utc_datetime(&at(utc))
        .with_timezone(&Local)
        .naive_local()
}

fn by_name(photos: &[Photo]) -> HashMap<String, (NaiveDateTime, TimeSource)> {
    photos
        .iter()
        .map(|p| {
            let name = p.path.file_name().unwrap().to_string_lossy().into_owned();
            (name, (p.taken_at, p.time_source))
        })
        .collect()
}

#[test]
fn dates_photos_by_their_takeout_date_files() {
    let dir = album();

    let timeline = build_timeline(dir.path(), |_| {}).unwrap();
    let photos = by_name(&timeline.photos);

    let takeout = TimeSource::Takeout(TakeoutUse::MissingExif);
    let expected = [
        ("no-exif.jpg", local("2024-06-01 10:00:00"), takeout),
        ("IMG_1.jpg", at("2024-06-01 12:00:05"), TimeSource::Exif),
        ("IMG_1(1).jpg", local("2024-06-01 10:00:08"), takeout),
        (
            "scan_1976.jpg",
            local("1976-07-01 10:00:00"),
            TimeSource::Takeout(TakeoutUse::Corrected),
        ),
        (
            "reset_clock.jpg",
            local("2024-06-02 08:00:00"),
            TimeSource::Takeout(TakeoutUse::ImplausibleExif),
        ),
        ("old_export.jpg", local("2024-06-03 08:00:00"), takeout),
        (
            "Screenshot_20240604-120000_Some_Long_App.jpg",
            local("2024-06-04 10:00:00"),
            takeout,
        ),
        ("renamed.jpg", local("2024-06-05 10:00:00"), takeout),
    ];
    for (name, time, source) in expected {
        assert_eq!(photos[name], (time, source), "{name}");
    }
    assert_eq!(
        photos["no-sidecar_reset-clock.jpg"].1,
        TimeSource::FileModified
    );
    assert_eq!(timeline.without_date(), 1);
}

#[test]
fn a_rejected_and_restored_photo_keeps_its_takeout_date() {
    let dir = album();
    let photo = dir.path().join("scan_1976.jpg");

    let rejected = reject(&photo).unwrap();
    let in_odrzucone = read_photo(&rejected.path).unwrap();
    restore(&rejected.path, &photo, &rejected.extra, true).unwrap();
    let back = read_photo(&photo).unwrap();

    let year_1976 = NaiveDate::from_ymd_opt(1976, 7, 1).unwrap();
    for read in [in_odrzucone, back] {
        assert_eq!(read.taken_at.date(), year_1976);
        assert_eq!(read.time_source, TimeSource::Takeout(TakeoutUse::Corrected));
    }
}
