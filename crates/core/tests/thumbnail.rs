// Test helpers may panic freely.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};

use image::{DynamicImage, GenericImageView, GrayImage, Luma, Rgb, RgbImage};
use sito_core::metadata::Orientation;
use sito_core::thumbnail::{
    DEFAULT_MAX_EDGE, ImageSource, ThumbnailError, render_image, render_thumbnail,
};
use tempfile::TempDir;

const RED: Rgb<u8> = Rgb([220, 20, 20]);
const BLUE: Rgb<u8> = Rgb([20, 20, 220]);

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/exif")
}

/// Writes a `width` × `height` blue JPEG whose top-left quarter is red.
fn write_marked_photo(dir: &Path, width: u32, height: u32) -> PathBuf {
    let image = RgbImage::from_fn(width, height, |x, y| {
        if x < width / 4 && y < height / 4 {
            RED
        } else {
            BLUE
        }
    });
    let path = dir.join(format!("photo_{width}x{height}.jpg"));
    image.save(&path).unwrap();
    path
}

fn decode(bytes: &[u8]) -> DynamicImage {
    image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg).unwrap()
}

/// Which corner of `image` holds the red marker.
fn red_corner(image: &DynamicImage) -> &'static str {
    let (w, h) = image.dimensions();
    let corners = [
        ("top-left", 2, 2),
        ("top-right", w - 3, 2),
        ("bottom-left", 2, h - 3),
        ("bottom-right", w - 3, h - 3),
    ];
    let red: Vec<_> = corners
        .iter()
        .filter(|(_, x, y)| {
            let [r, _, b, _] = image.get_pixel(*x, *y).0;
            r > 150 && b < 100
        })
        .map(|(name, _, _)| *name)
        .collect();
    assert_eq!(red.len(), 1, "exactly one red corner expected, got {red:?}");
    red[0]
}

#[test]
fn scales_large_photos_down_keeping_the_aspect_ratio() {
    let dir = TempDir::new().unwrap();
    let landscape = write_marked_photo(dir.path(), 1600, 1200);
    let portrait = write_marked_photo(dir.path(), 900, 1600);

    let landscape = decode(&render_thumbnail(&landscape, Orientation::NORMAL, 400).unwrap());
    let portrait = decode(&render_thumbnail(&portrait, Orientation::NORMAL, 400).unwrap());

    assert_eq!(landscape.dimensions(), (400, 300));
    assert_eq!(portrait.dimensions(), (225, 400));
}

#[test]
fn never_scales_small_photos_up() {
    let path = fixtures().join("pixel_2024-06-01_12-00-00.jpg");

    let thumbnail =
        decode(&render_thumbnail(&path, Orientation::NORMAL, DEFAULT_MAX_EDGE).unwrap());

    assert_eq!(thumbnail.dimensions(), (16, 12));
}

#[test]
fn turns_photos_upright_for_every_orientation() {
    let dir = TempDir::new().unwrap();
    let path = write_marked_photo(dir.path(), 1600, 1200);
    let cases = [
        (1, (400, 300), "top-left"),
        (2, (400, 300), "top-right"),
        (3, (400, 300), "bottom-right"),
        (4, (400, 300), "bottom-left"),
        (5, (300, 400), "top-left"),
        (6, (300, 400), "top-right"),
        (7, (300, 400), "bottom-right"),
        (8, (300, 400), "bottom-left"),
    ];

    for (orientation, size, corner) in cases {
        let bytes = render_thumbnail(&path, Orientation::from_exif(orientation), 400).unwrap();
        let thumbnail = decode(&bytes);

        assert_eq!(thumbnail.dimensions(), size, "orientation {orientation}");
        assert_eq!(red_corner(&thumbnail), corner, "orientation {orientation}");
    }
}

#[test]
fn handles_grayscale_photos() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("gray.jpg");
    GrayImage::from_pixel(800, 600, Luma([128]))
        .save(&path)
        .unwrap();

    let thumbnail = decode(&render_thumbnail(&path, Orientation::NORMAL, 400).unwrap());

    assert_eq!(thumbnail.dimensions(), (400, 300));
}

#[test]
fn renders_an_image_held_in_memory() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("gray.jpg");
    GrayImage::from_pixel(800, 600, Luma([128]))
        .save(&path)
        .unwrap();
    let bytes = fs::read(&path).unwrap();

    let source = ImageSource::Bytes {
        bytes: &bytes,
        path: &path,
    };
    let thumbnail = decode(&render_image(source, Orientation::from_exif(6), 400).unwrap());

    assert_eq!(thumbnail.dimensions(), (300, 400));
}

#[test]
fn reports_files_that_are_not_images() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("broken.jpg");
    fs::write(&path, b"this is not a JPEG").unwrap();

    let err = render_thumbnail(&path, Orientation::NORMAL, 400).unwrap_err();

    assert!(matches!(err, ThumbnailError::Decode { .. }), "{err:?}");
}

#[test]
fn reports_missing_files() {
    let dir = TempDir::new().unwrap();

    let err = render_thumbnail(&dir.path().join("gone.jpg"), Orientation::NORMAL, 400).unwrap_err();

    assert!(matches!(err, ThumbnailError::Io { .. }), "{err:?}");
}
