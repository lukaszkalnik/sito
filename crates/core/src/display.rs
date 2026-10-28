//! The image the viewer shows for a photo: a JPEG a browser can display,
//! whatever the photo's file format.

use std::path::{Path, PathBuf};

use crate::metadata::Orientation;
use crate::raw::{RawError, read_preview, with_orientation};
use crate::scan::{PhotoKind, photo_kind};

/// Why a photo cannot be shown.
#[derive(Debug, thiserror::Error)]
pub enum DisplayError {
    /// The file could not be read.
    #[error("cannot read {path}")]
    Io {
        /// The photo.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The RAW file has no usable preview.
    #[error("cannot show {path}")]
    Raw {
        /// The photo.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: RawError,
    },
    /// The file is not a photo Sito reads.
    #[error("{0} is not a photo")]
    NotAPhoto(PathBuf),
}

/// A JPEG of the photo at `path` for full-size viewing, carrying the photo's
/// `orientation` so the viewer turns it upright: a JPEG's own bytes, or a
/// RAW file's embedded preview.
///
/// # Errors
///
/// Fails if the file cannot be read or holds nothing a browser can show.
pub fn display_jpeg(path: &Path, orientation: Orientation) -> Result<Vec<u8>, DisplayError> {
    match photo_kind(path) {
        Some(PhotoKind::Jpeg) => std::fs::read(path).map_err(|source| DisplayError::Io {
            path: path.to_path_buf(),
            source,
        }),
        Some(PhotoKind::Raw(format)) => read_preview(path, format)
            .map(|preview| with_orientation(preview, orientation))
            .map_err(|source| DisplayError::Raw {
                path: path.to_path_buf(),
                source,
            }),
        None => Err(DisplayError::NotAPhoto(path.to_path_buf())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(dir: &str, name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(dir)
            .join(name)
    }

    #[test]
    fn shows_a_jpeg_as_it_is() {
        let path = fixture("exif", "rotated_orientation-6.jpg");

        let shown = display_jpeg(&path, Orientation::from_exif(6)).unwrap();

        assert_eq!(shown, std::fs::read(&path).unwrap());
    }

    #[test]
    fn shows_a_raw_file_by_its_preview() {
        let shown = display_jpeg(&fixture("raw", "olympus.orf"), Orientation::NORMAL).unwrap();

        assert!(shown.starts_with(&[0xFF, 0xD8]));
    }

    #[test]
    fn a_raw_file_without_preview_cannot_be_shown() {
        let path = fixture("raw", "olympus-no-preview.ORF");

        assert!(matches!(
            display_jpeg(&path, Orientation::NORMAL),
            Err(DisplayError::Raw { .. })
        ));
    }
}
