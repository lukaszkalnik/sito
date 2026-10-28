//! Camera RAW files: their metadata and the JPEG preview cameras embed in
//! them. RAW data itself is never developed; a file without a usable
//! preview cannot be shown.

mod format;
mod maker_note;
mod orientation;
mod preview;

use std::path::Path;

pub use format::RawFormat;
pub use orientation::with_orientation;
pub use preview::read_preview;

use crate::metadata::{PhotoMetadata, read_exif_metadata};
use crate::tiff::read_tiff_metadata;

/// Why a RAW file could not be shown.
#[derive(Debug, thiserror::Error)]
pub enum RawError {
    /// The file could not be read.
    #[error("cannot read the RAW file")]
    Io(#[from] std::io::Error),
    /// The file holds no JPEG preview a browser can show.
    #[error("the RAW file has no usable preview")]
    NoPreview,
}

/// Reads the metadata of the RAW file at `path`.
///
/// Never fails: unreadable files and missing values yield
/// [`PhotoMetadata::default`] or partially filled metadata.
#[must_use]
pub fn read_raw_metadata(path: &Path, format: RawFormat) -> PhotoMetadata {
    let is_jpeg = std::fs::File::open(path)
        .map(std::io::BufReader::new)
        .is_ok_and(|mut file| preview::is_jpeg(&mut file));
    if format.is_tiff_based() && !is_jpeg {
        read_tiff_metadata(path)
    } else {
        // `nom-exif` reads Fujifilm's RAF container and JPEGs itself.
        read_exif_metadata(path)
    }
}
