//! Real RAW files from `samples/raw/` (downloaded by
//! `scripts/get-raw-samples.py`, not committed). Run with
//! `cargo test --test raw_samples -- --ignored --nocapture`.

// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use image::GenericImageView;
use sito_core::metadata::read_photo_metadata;
use sito_core::raw::{RawFormat, read_preview};

#[test]
#[ignore = "needs samples/raw/ from scripts/get-raw-samples.py"]
fn reads_real_raw_files() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/raw");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| RawFormat::from_path(path).is_some())
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no samples in {}", dir.display());

    let mut with_preview = 0;
    for path in &files {
        let format = RawFormat::from_path(path).unwrap();
        let meta = read_photo_metadata(path);
        let preview = read_preview(path, format)
            .ok()
            .and_then(|bytes| image::load_from_memory(&bytes).ok())
            .map(|image| image.dimensions());
        with_preview += usize::from(preview.is_some());
        println!(
            "{:<75} date {:<20} camera {:<35} orientation {} preview {:?}",
            path.file_name().unwrap().to_string_lossy(),
            meta.captured.map_or("-".into(), |c| c.local.to_string()),
            meta.camera.unwrap_or_default(),
            meta.orientation.value(),
            preview,
        );
    }
    assert!(
        with_preview * 4 >= files.len() * 3,
        "only {with_preview} of {} samples have a preview",
        files.len()
    );
}
