//! A photo's resolution, read without decoding the image.

use std::path::Path;

use image::ImageReader;

use crate::metadata::Orientation;
use crate::scan::{PhotoKind, photo_kind};

/// Width and height of a photo in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Dimensions {
    /// The dimensions as the photo is shown after applying `orientation`:
    /// width and height swap for the orientations that turn it by 90°
    /// (EXIF 5–8).
    #[must_use]
    pub fn upright(self, orientation: Orientation) -> Self {
        if orientation.value() >= 5 {
            Self {
                width: self.height,
                height: self.width,
            }
        } else {
            self
        }
    }
}

/// The stored dimensions of the photo at `path`, before its EXIF orientation,
/// from the image header of a JPEG or TIFF (nothing is decoded). `None` if the
/// file cannot be read, and for RAW files: their EXIF image size is missing or
/// is the preview's in most files, and their own headers differ per camera.
#[must_use]
pub fn read_dimensions(path: &Path) -> Option<Dimensions> {
    match photo_kind(path)? {
        PhotoKind::Jpeg | PhotoKind::Tiff => header_dimensions(path),
        PhotoKind::Raw(_) => None,
    }
}

fn header_dimensions(path: &Path) -> Option<Dimensions> {
    let (width, height) = ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()?;
    Some(Dimensions { width, height })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn fixture(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(path)
    }

    const LANDSCAPE: Dimensions = Dimensions {
        width: 16,
        height: 12,
    };

    #[test]
    fn reads_a_jpeg_header() {
        assert_eq!(
            read_dimensions(&fixture("exif/pixel_2024-06-01_12-00-00.jpg")),
            Some(LANDSCAPE)
        );
    }

    #[test]
    fn reads_a_tiff_header() {
        assert_eq!(
            read_dimensions(&fixture("tiff/no-date.tif")),
            Some(Dimensions {
                width: 40,
                height: 30
            })
        );
    }

    #[test]
    fn has_none_for_a_missing_file_or_another_kind() {
        assert_eq!(read_dimensions(&fixture("exif/gone.jpg")), None);
        assert_eq!(read_dimensions(&fixture("README.md")), None);
    }

    #[test]
    fn swaps_width_and_height_for_a_photo_turned_by_90_degrees() {
        let portrait = Dimensions {
            width: 12,
            height: 16,
        };
        assert_eq!(LANDSCAPE.upright(Orientation::NORMAL), LANDSCAPE);
        assert_eq!(LANDSCAPE.upright(Orientation::from_exif(3)), LANDSCAPE);
        assert_eq!(LANDSCAPE.upright(Orientation::from_exif(6)), portrait);
        assert_eq!(LANDSCAPE.upright(Orientation::from_exif(8)), portrait);
    }

    #[test]
    fn has_none_for_a_raw_file() {
        assert_eq!(read_dimensions(&fixture("raw/canon.cr2")), None);
    }
}
