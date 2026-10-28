//! TIFF structures, which TIFF images and most RAW formats are built on.

mod metadata;
mod reader;

pub use metadata::read_tiff_metadata;
pub use reader::{Endian, Entry, Ifd, MAX_BLOCK, TiffError, TiffReader, tag};
