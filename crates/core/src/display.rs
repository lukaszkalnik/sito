//! The image the viewer shows for a photo: a JPEG a browser can display,
//! whatever the photo's file format.

use std::path::{Path, PathBuf};

use crate::limited_read::{LimitedReadError, MAX_JPEG_BYTES, read_limited};
use crate::raw::{RawError, read_preview, with_orientation};
use crate::scan::{PhotoKind, photo_kind};
use crate::thumbnail::{ThumbnailCache, ThumbnailError};
use crate::timeline::Photo;

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
    /// The file is too large to be read whole for viewing.
    #[error("{0} is too large to show")]
    TooLarge(PathBuf),
    /// The RAW file has no usable preview.
    #[error("cannot show {path}")]
    Raw {
        /// The photo.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: RawError,
    },
    /// The photo could not be converted for viewing (a TIFF).
    #[error("cannot convert {path}")]
    Convert {
        /// The photo.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: Box<ThumbnailError>,
    },
    /// The file is not a photo Sito reads.
    #[error("{0} is not a photo")]
    NotAPhoto(PathBuf),
}

/// A JPEG of `photo` for full-size viewing, carrying the photo's orientation
/// so the viewer turns it upright: a JPEG's own bytes, a RAW file's embedded
/// preview, or a TIFF converted once and kept in `cache`.
///
/// # Errors
///
/// Fails if the file cannot be read or holds nothing a browser can show.
pub fn display_jpeg(photo: &Photo, cache: &ThumbnailCache) -> Result<Vec<u8>, DisplayError> {
    let path = photo.path.as_path();
    let orientation = photo.orientation();
    match photo_kind(path) {
        Some(PhotoKind::Jpeg) => read_jpeg(path),
        Some(PhotoKind::Raw(format)) => read_preview(path, format)
            .map(|preview| with_orientation(preview, orientation))
            .map_err(|source| raw_error(path, source)),
        Some(PhotoKind::Tiff) => {
            cache
                .display_jpeg_for(photo)
                .map_err(|source| DisplayError::Convert {
                    path: path.to_path_buf(),
                    source: Box::new(source),
                })
        }
        None => Err(DisplayError::NotAPhoto(path.to_path_buf())),
    }
}

/// The JPEG at `path`, whole.
fn read_jpeg(path: &Path) -> Result<Vec<u8>, DisplayError> {
    read_limited(path, MAX_JPEG_BYTES).map_err(|err| match err {
        LimitedReadError::Io(source) => DisplayError::Io {
            path: path.to_path_buf(),
            source,
        },
        LimitedReadError::TooLarge { .. } => DisplayError::TooLarge(path.to_path_buf()),
    })
}

/// Why the RAW file at `path` cannot be shown.
fn raw_error(path: &Path, source: RawError) -> DisplayError {
    match source {
        RawError::TooLarge => DisplayError::TooLarge(path.to_path_buf()),
        RawError::Io(_) | RawError::NoPreview => DisplayError::Raw {
            path: path.to_path_buf(),
            source,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::Orientation;
    use crate::timeline::ExifInfo;

    fn fixture(dir: &str, name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(dir)
            .join(name)
    }

    fn photo(path: PathBuf, orientation: u16) -> Photo {
        let meta = std::fs::metadata(&path).unwrap();
        Photo {
            path,
            size: meta.len(),
            modified: meta.modified().ok(),
            taken_at: chrono::NaiveDateTime::default(),
            time_source: crate::timeline::TimeSource::Exif,
            exif: ExifInfo::Read {
                orientation: Orientation::from_exif(orientation),
                camera: None,
            },
            sidecar: None,
            companion: None,
        }
    }

    fn show(photo: &Photo) -> Result<Vec<u8>, DisplayError> {
        let dir = tempfile::TempDir::new().unwrap();
        display_jpeg(photo, &ThumbnailCache::new(dir.path()))
    }

    #[test]
    fn shows_a_jpeg_as_it_is() {
        let path = fixture("exif", "rotated_orientation-6.jpg");

        let shown = show(&photo(path.clone(), 6)).unwrap();

        assert_eq!(shown, std::fs::read(&path).unwrap());
    }

    #[test]
    fn shows_a_raw_file_by_its_preview() {
        let shown = show(&photo(fixture("raw", "olympus.orf"), 1)).unwrap();

        assert!(shown.starts_with(&[0xFF, 0xD8]));
    }

    #[test]
    fn a_raw_file_without_preview_cannot_be_shown() {
        let path = fixture("raw", "olympus-no-preview.ORF");

        assert!(matches!(
            show(&photo(path, 1)),
            Err(DisplayError::Raw { .. })
        ));
    }

    /// A file just over the size limit that starts like a JPEG. It is
    /// sparse: only its length is set, nothing large is written.
    fn oversized_jpeg(dir: &tempfile::TempDir, name: &str) -> PathBuf {
        use std::io::Write;

        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&[0xFF, 0xD8, 0xFF]).unwrap();
        file.set_len(MAX_JPEG_BYTES + 1).unwrap();
        path
    }

    #[test]
    fn a_jpeg_over_the_size_limit_is_not_read() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = oversized_jpeg(&dir, "huge.jpg");

        assert!(matches!(
            show(&photo(path.clone(), 1)),
            Err(DisplayError::TooLarge(refused)) if refused == path
        ));
    }

    #[test]
    fn a_jpeg_with_a_raw_name_over_the_size_limit_is_not_read() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = oversized_jpeg(&dir, "huge.orf");

        assert!(matches!(
            show(&photo(path.clone(), 1)),
            Err(DisplayError::TooLarge(refused)) if refused == path
        ));
    }
}
