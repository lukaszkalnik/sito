//! Camera RAW files: their metadata and the JPEG preview cameras embed in
//! them. RAW data itself is never developed; a file without a usable
//! preview cannot be shown.

// Bytes from files are untrusted here: nothing may index or compute its
// way into a panic (inherited by the submodules).
#![cfg_attr(
    not(test),
    warn(clippy::indexing_slicing, clippy::arithmetic_side_effects)
)]

mod format;
mod maker_note;
mod orientation;
mod preview;

use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

pub use format::RawFormat;
pub use orientation::with_orientation;
pub use preview::{read_preview, read_preview_from};

use crate::metadata::{PhotoMetadata, read_exif_metadata_from};
use crate::tiff::read_tiff_metadata_from;

/// Why a RAW file could not be shown.
#[derive(Debug, thiserror::Error)]
pub enum RawError {
    /// The file could not be read.
    #[error("cannot read the RAW file")]
    Io(#[from] std::io::Error),
    /// The file is a JPEG too large to be read whole for viewing.
    #[error("the file is too large to show")]
    TooLarge,
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
    std::fs::File::open(path)
        .map(|file| read_raw_metadata_from(BufReader::new(file), format))
        .unwrap_or_default()
}

/// Like [`read_raw_metadata`], for a RAW file held in `reader`.
#[must_use]
pub fn read_raw_metadata_from(mut reader: impl Read + Seek, format: RawFormat) -> PhotoMetadata {
    let is_jpeg = preview::is_jpeg(&mut reader);
    if reader.seek(SeekFrom::Start(0)).is_err() {
        return PhotoMetadata::default();
    }
    if format.is_tiff_based() && !is_jpeg {
        read_tiff_metadata_from(reader)
    } else {
        // `nom-exif` reads Fujifilm's RAF container and JPEGs itself.
        read_exif_metadata_from(reader)
    }
}
