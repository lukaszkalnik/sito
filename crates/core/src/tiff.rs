//! TIFF structures, which TIFF images and most RAW formats are built on.

// Bytes from files are untrusted here: nothing may index or compute its
// way into a panic (inherited by the submodules).
#![cfg_attr(
    not(test),
    warn(clippy::indexing_slicing, clippy::arithmetic_side_effects)
)]

mod metadata;
mod reader;

pub use metadata::read_tiff_metadata;
pub use reader::{Endian, Entry, Ifd, MAX_BLOCK, TiffError, TiffReader, tag};
