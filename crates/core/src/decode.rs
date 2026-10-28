//! Turning a photo file of any kind into pixels: for thumbnails, the viewer's
//! display images and image analysis.

use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use image::{DynamicImage, GrayImage, ImageError, ImageReader, Limits, RgbImage};

use crate::raw::{RawError, read_preview};
use crate::scan::{PhotoKind, photo_kind};

/// Most memory one TIFF decode may take. Scans can be huge (a 16-bit
/// 8000 × 6000 scan needs about 300 MB); larger ones are refused.
pub const TIFF_MEMORY_LIMIT: u64 = 1024 * 1024 * 1024;

/// Lets only one TIFF decode run at a time, so two thumbnail workers never
/// hold two huge scans in memory at once.
static TIFF_DECODE: Mutex<()> = Mutex::new(());

/// Why a photo could not be decoded.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// The file could not be read.
    #[error("cannot read {path}")]
    Io {
        /// The photo.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The file is not a decodable image.
    #[error("cannot decode {path}")]
    Image {
        /// The photo.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: ImageError,
    },
    /// A RAW file's embedded preview could not be read.
    #[error("cannot read the preview of {path}")]
    Raw {
        /// The photo.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: RawError,
    },
}

/// Where an encoded image comes from.
#[derive(Debug, Clone, Copy)]
pub enum ImageSource<'a> {
    /// An image file.
    Path(&'a Path),
    /// An image already in memory (e.g. the preview embedded in a RAW file),
    /// with the path of the photo it belongs to, for error messages.
    Bytes {
        /// The encoded image.
        bytes: &'a [u8],
        /// The photo the image belongs to.
        path: &'a Path,
    },
}

impl ImageSource<'_> {
    /// The photo the image belongs to.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Path(path) | Self::Bytes { path, .. } => path,
        }
    }
}

/// The pixels of the photo at `path`, as its kind needs: a JPEG directly, a
/// RAW file through its embedded preview, a TIFF's first page (under
/// [`TIFF_MEMORY_LIMIT`]). With `max_edge`, a JPEG may be
/// decoded at a reduced size that still covers `max_edge` × `max_edge`; the
/// result is not resized further.
///
/// # Errors
///
/// Fails if the file cannot be read or decoded.
pub fn decode_photo(path: &Path, max_edge: Option<u32>) -> Result<DynamicImage, DecodeError> {
    match photo_kind(path) {
        Some(PhotoKind::Raw(format)) => {
            let preview = read_preview(path, format).map_err(|source| DecodeError::Raw {
                path: path.to_path_buf(),
                source,
            })?;
            decode_image(
                ImageSource::Bytes {
                    bytes: &preview,
                    path,
                },
                max_edge,
            )
        }
        Some(PhotoKind::Tiff) => decode_tiff(path, TIFF_MEMORY_LIMIT),
        Some(PhotoKind::Jpeg) | None => decode_image(ImageSource::Path(path), max_edge),
    }
}

/// The first page of the TIFF at `path`, decoded with at most `memory_limit`
/// bytes, one TIFF at a time.
pub(crate) fn decode_tiff(path: &Path, memory_limit: u64) -> Result<DynamicImage, DecodeError> {
    let _one_at_a_time = TIFF_DECODE.lock().unwrap_or_else(PoisonError::into_inner);
    let io_error = |source| DecodeError::Io {
        path: path.to_path_buf(),
        source,
    };
    let mut reader = ImageReader::open(path)
        .map_err(io_error)?
        .with_guessed_format()
        .map_err(io_error)?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(memory_limit);
    reader.limits(limits);
    reader.decode().map_err(|source| DecodeError::Image {
        path: path.to_path_buf(),
        source,
    })
}

/// The pixels of an encoded image, decoded at a reduced size covering
/// `max_edge` when the format allows (JPEG), else at full size.
///
/// # Errors
///
/// Fails if the image cannot be read or decoded.
pub fn decode_image(
    source: ImageSource<'_>,
    max_edge: Option<u32>,
) -> Result<DynamicImage, DecodeError> {
    match max_edge.and_then(|edge| decode_scaled(source, edge)) {
        Some(image) => Ok(image),
        None => decode_full(source),
    }
}

/// Fast path: lets the JPEG decoder skip detail (DCT scaling by up to 1/8),
/// so a 12 MP photo is never decoded at full size. Returns `None` for
/// anything it cannot handle; the caller then falls back to a full decode.
fn decode_scaled(source: ImageSource<'_>, max_edge: u32) -> Option<DynamicImage> {
    match source {
        ImageSource::Path(path) => {
            decode_scaled_from(BufReader::new(File::open(path).ok()?), max_edge)
        }
        ImageSource::Bytes { bytes, .. } => decode_scaled_from(Cursor::new(bytes), max_edge),
    }
}

fn decode_scaled_from(reader: impl Read, max_edge: u32) -> Option<DynamicImage> {
    let mut decoder = jpeg_decoder::Decoder::new(reader);
    decoder.read_info().ok()?;
    let info = decoder.info()?;

    let (width, height) = fit_inside(u32::from(info.width), u32::from(info.height), max_edge);
    let (width, height) = decoder
        .scale(u16::try_from(width).ok()?, u16::try_from(height).ok()?)
        .ok()?;
    let pixels = decoder.decode().ok()?;

    let (width, height) = (u32::from(width), u32::from(height));
    match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            RgbImage::from_raw(width, height, pixels).map(DynamicImage::ImageRgb8)
        }
        jpeg_decoder::PixelFormat::L8 => {
            GrayImage::from_raw(width, height, pixels).map(DynamicImage::ImageLuma8)
        }
        jpeg_decoder::PixelFormat::L16 | jpeg_decoder::PixelFormat::CMYK32 => None,
    }
}

/// Slow path: full decode with the `image` crate.
fn decode_full(source: ImageSource<'_>) -> Result<DynamicImage, DecodeError> {
    let path = source.path();
    let io_error = |source| DecodeError::Io {
        path: path.to_path_buf(),
        source,
    };
    let image_error = |source| DecodeError::Image {
        path: path.to_path_buf(),
        source,
    };
    match source {
        ImageSource::Path(path) => ImageReader::open(path)
            .map_err(io_error)?
            .with_guessed_format()
            .map_err(io_error)?
            .decode()
            .map_err(image_error),
        ImageSource::Bytes { bytes, .. } => ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(io_error)?
            .decode()
            .map_err(image_error),
    }
}

/// Size of a `width` × `height` image scaled down to fit inside
/// `max_edge` × `max_edge`, keeping the aspect ratio. Never scales up.
pub(crate) fn fit_inside(width: u32, height: u32, max_edge: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= max_edge || longest == 0 {
        return (width, height);
    }
    let scale = |side: u32| {
        let scaled = u64::from(side) * u64::from(max_edge) / u64::from(longest);
        u32::try_from(scaled).unwrap_or(max_edge).max(1)
    };
    (scale(width), scale(height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_inside_keeps_aspect_ratio() {
        assert_eq!(fit_inside(4000, 3000, 400), (400, 300));
        assert_eq!(fit_inside(3000, 4000, 400), (300, 400));
        assert_eq!(fit_inside(1000, 1000, 400), (400, 400));
    }

    #[test]
    fn fit_inside_never_scales_up() {
        assert_eq!(fit_inside(16, 12, 400), (16, 12));
        assert_eq!(fit_inside(400, 10, 400), (400, 10));
    }

    #[test]
    fn fit_inside_keeps_at_least_one_pixel() {
        assert_eq!(fit_inside(8000, 1, 400), (400, 1));
        assert_eq!(fit_inside(0, 0, 400), (0, 0));
    }

    #[test]
    fn a_tiff_over_the_memory_limit_is_refused() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/tiff/rgb8_orientation-6.tif");

        // 40 × 30 RGB needs 3,600 bytes.
        assert!(decode_tiff(&path, 100).is_err());
        assert!(decode_tiff(&path, 1024 * 1024).is_ok());
    }

    #[test]
    fn large_rgb_jpegs_take_the_scaled_decode_path() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("big.jpg");
        RgbImage::from_pixel(1600, 1200, image::Rgb([10, 200, 10]))
            .save(&path)
            .unwrap();

        let image = decode_scaled(ImageSource::Path(&path), 400).unwrap();

        // 1/4 is the smallest DCT scale that still covers 400 × 300.
        assert_eq!((image.width(), image.height()), (400, 300));
    }
}
