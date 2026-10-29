// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use image::GenericImageView;
use sito_core::decode::{DecodeError, decode_photo};
use sito_core::metadata::{Orientation, read_photo_metadata};
use sito_core::thumbnail::{ThumbnailCache, render_photo};
use sito_core::timeline::{Photo, TimeSource, build_timeline};
use tempfile::TempDir;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/tiff")
        .join(name)
}

#[test]
fn reads_the_capture_time_orientation_and_scanner() {
    let meta = read_photo_metadata(&fixture("rgb8_orientation-6.tif"));

    assert_eq!(
        meta.captured.unwrap().local.to_string(),
        "2024-06-12 09:00:00"
    );
    assert_eq!(meta.orientation, Orientation::from_exif(6));
    assert_eq!(meta.camera.as_deref(), Some("EPSON Perfection V600"));
}

#[test]
fn falls_back_to_the_tiff_date_time() {
    let meta = read_photo_metadata(&fixture("gray8_datetime-only.tif"));

    assert_eq!(
        meta.captured.unwrap().local.to_string(),
        "2024-06-13 10:00:00"
    );
}

#[test]
fn a_scan_without_a_date_has_none() {
    assert_eq!(read_photo_metadata(&fixture("no-date.tif")).captured, None);
}

#[test]
fn decodes_every_kind_of_tiff() {
    for name in [
        "rgb8_orientation-6.tif",
        "gray8_datetime-only.tif",
        "gray16.tif",
        "cmyk.tif",
        "no-date.tif",
    ] {
        let image = decode_photo(&fixture(name), None).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(image.dimensions(), (40, 30), "{name}");
    }
}

#[test]
fn shows_the_first_page_of_a_multi_page_tiff() {
    let image = decode_photo(&fixture("two-pages.tif"), None).unwrap();

    assert_eq!(image.dimensions(), (40, 30));
}

#[test]
fn a_broken_tiff_is_an_error() {
    let err = decode_photo(&fixture("broken.tif"), None).unwrap_err();

    assert!(matches!(err, DecodeError::Image { .. }), "{err:?}");
}

#[test]
fn thumbnails_are_turned_upright() {
    let thumbnail = render_photo(
        &fixture("rgb8_orientation-6.tif"),
        Orientation::from_exif(6),
        400,
    )
    .unwrap();

    let image = image::load_from_memory(&thumbnail).unwrap();
    assert_eq!(image.dimensions(), (30, 40));
}

fn photo(path: PathBuf) -> Photo {
    let meta = std::fs::metadata(&path).unwrap();
    Photo {
        path,
        size: meta.len(),
        modified: meta.modified().ok(),
        taken_at: chrono::NaiveDateTime::default(),
        time_source: TimeSource::Exif,
        orientation: Orientation::from_exif(6),
        camera: None,
        sidecar: None,
        companion: None,
    }
}

#[test]
fn the_viewer_jpeg_is_made_once_and_kept() {
    let cache_dir = TempDir::new().unwrap();
    let cache = ThumbnailCache::new(cache_dir.path());
    let scan = photo(fixture("rgb8_orientation-6.tif"));

    let first = cache.display_jpeg_for(&scan).unwrap();

    let stored = cache.display_path_for(&scan);
    assert_eq!(std::fs::read(&stored).unwrap(), first);
    let image = image::load_from_memory(&first).unwrap();
    assert_eq!(image.dimensions(), (40, 30), "full size, turned by EXIF");
    // A second call reads the stored file.
    std::fs::write(&stored, b"cached").unwrap();
    assert_eq!(cache.display_jpeg_for(&scan).unwrap(), b"cached");
}

#[test]
fn cleaning_keeps_the_viewer_jpegs_of_known_photos() {
    let cache_dir = TempDir::new().unwrap();
    let cache = ThumbnailCache::new(cache_dir.path());
    let scan = photo(fixture("rgb8_orientation-6.tif"));
    cache.display_jpeg_for(&scan).unwrap();

    let report = cache.clean(std::slice::from_ref(&scan), u64::MAX).unwrap();

    assert_eq!(report.orphans_removed, 0);
    assert!(cache.display_path_for(&scan).is_file());
}

#[test]
fn tiffs_are_on_the_timeline() {
    let timeline = build_timeline(&fixture(""), |_| {}).unwrap();

    // Every fixture but the broken one is a photo; the broken one is listed
    // too (it is a .tif file) and dated by the file.
    assert_eq!(timeline.photos.len(), 7);
}

#[test]
fn viewer_jpegs_of_large_scans_are_at_most_4096_px() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("wide-scan.tif");
    image::RgbImage::from_pixel(6000, 3000, image::Rgb([90, 140, 200]))
        .save(&path)
        .unwrap();
    let mut scan = photo(path);
    scan.orientation = Orientation::NORMAL;

    let jpeg = ThumbnailCache::new(dir.path().join("cache"))
        .display_jpeg_for(&scan)
        .unwrap();

    let image = image::load_from_memory(&jpeg).unwrap();
    assert_eq!(image.dimensions(), (4096, 2048));
}
