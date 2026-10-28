// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use image::GenericImageView;
use sito_core::metadata::{Orientation, read_exif_metadata, read_photo_metadata};
use sito_core::raw::{RawError, RawFormat, read_preview, with_orientation};
use sito_core::thumbnail::render_photo;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/raw")
        .join(name)
}

/// Every synthetic RAW fixture: file, camera, EXIF orientation.
const FILES: [(&str, &str, u16); 7] = [
    ("olympus.orf", "OLYMPUS IMAGING CORP. E-M10", 6),
    ("canon.cr2", "Canon EOS 80D", 8),
    ("nikon.nef", "NIKON CORPORATION NIKON D750", 1),
    ("sony.arw", "SONY ILCE-7M3", 1),
    ("panasonic.rw2", "Panasonic DC-G9", 1),
    ("google.dng", "Google Pixel 8", 1),
    ("fujifilm.raf", "FUJIFILM X-T3", 1),
];

#[test]
fn reads_the_metadata_of_every_raw_layout() {
    for (name, camera, orientation) in FILES {
        let meta = read_photo_metadata(&fixture(name));

        let captured = meta.captured.unwrap_or_else(|| panic!("{name}: no date"));
        assert_eq!(captured.local.to_string(), "2024-06-10 10:00:00", "{name}");
        assert_eq!(meta.camera.as_deref(), Some(camera), "{name}");
        assert_eq!(
            meta.orientation,
            Orientation::from_exif(orientation),
            "{name}"
        );
    }
}

#[test]
fn finds_the_largest_embedded_preview() {
    for (name, ..) in FILES {
        let path = fixture(name);
        let format = RawFormat::from_path(&path).unwrap();

        let preview = read_preview(&path, format).unwrap_or_else(|e| panic!("{name}: {e}"));

        // The 64×48 preview, not Olympus' 16×12 thumbnail.
        let image = image::load_from_memory(&preview).unwrap();
        assert_eq!(image.dimensions(), (64, 48), "{name}");
    }
}

#[test]
fn a_raw_file_without_preview_has_none() {
    let path = fixture("olympus-no-preview.ORF");

    let err = read_preview(&path, RawFormat::Orf).unwrap_err();

    assert!(matches!(err, RawError::NoPreview));
}

#[test]
fn thumbnails_come_from_the_preview_turned_upright() {
    let thumbnail = render_photo(&fixture("olympus.orf"), Orientation::from_exif(6), 400).unwrap();

    let image = image::load_from_memory(&thumbnail).unwrap();
    assert_eq!(image.dimensions(), (48, 64));
}

#[test]
fn the_preview_for_viewing_carries_the_orientation() {
    let path = fixture("olympus.orf");
    let preview = read_preview(&path, RawFormat::Orf).unwrap();

    let shown = with_orientation(preview, Orientation::from_exif(6));

    // Read back as a JPEG file, the way the viewer gets it.
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("shown.jpg");
    std::fs::write(&file, shown).unwrap();
    let orientation = read_exif_metadata(&file).orientation;
    assert_eq!(orientation, Orientation::from_exif(6));
}

#[test]
fn raw_files_are_on_the_timeline_with_their_capture_times() {
    let timeline = sito_core::timeline::build_timeline(&fixture(""), |_| {}).unwrap();

    assert_eq!(timeline.photos.len(), FILES.len() + 1);
    for photo in &timeline.photos {
        assert_eq!(
            photo.time_source,
            sito_core::timeline::TimeSource::Exif,
            "{}",
            photo.path.display()
        );
    }
}
