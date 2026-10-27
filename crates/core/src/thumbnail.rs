//! Small, upright JPEG previews of photos.

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, GrayImage, ImageError, ImageReader, RgbImage};

use crate::metadata::Orientation;

/// Default longest edge of a thumbnail in pixels: covers a 160×120 tile on a
/// 2× display.
pub const DEFAULT_MAX_EDGE: u32 = 400;

/// JPEG quality of thumbnails (1–100).
const JPEG_QUALITY: u8 = 80;

/// Why a thumbnail could not be made.
#[derive(Debug, thiserror::Error)]
pub enum ThumbnailError {
    /// The photo or the cache could not be read or written.
    #[error("cannot access {path}")]
    Io {
        /// File that failed.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The photo is not a decodable image.
    #[error("cannot decode {path}")]
    Decode {
        /// Photo that failed.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: ImageError,
    },
    /// The thumbnail could not be encoded.
    #[error("cannot encode thumbnail of {path}")]
    Encode {
        /// Photo whose thumbnail failed.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: ImageError,
    },
}

/// Renders a JPEG thumbnail of the photo at `path` that fits inside
/// `max_edge` × `max_edge`, turned upright according to `orientation`.
/// Small photos are never upscaled.
///
/// # Errors
///
/// Returns [`ThumbnailError`] if the photo cannot be read or decoded.
pub fn render_thumbnail(
    path: &Path,
    orientation: Orientation,
    max_edge: u32,
) -> Result<Vec<u8>, ThumbnailError> {
    let image = match decode_scaled(path, max_edge) {
        Some(image) => image,
        None => decode_full(path)?,
    };

    let mut image = if image.width().max(image.height()) > max_edge {
        image.resize(max_edge, max_edge, FilterType::Triangle)
    } else {
        image
    };
    if let Some(o) = u8::try_from(orientation.value())
        .ok()
        .and_then(image::metadata::Orientation::from_exif)
    {
        image.apply_orientation(o);
    }

    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(Cursor::new(&mut bytes), JPEG_QUALITY)
        .encode_image(&image.into_rgb8())
        .map_err(|source| ThumbnailError::Encode {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(bytes)
}

/// Fast path: lets the JPEG decoder skip detail (DCT scaling by up to 1/8),
/// so a 12 MP photo is never decoded at full size. Returns `None` for
/// anything it cannot handle; the caller then falls back to a full decode.
fn decode_scaled(path: &Path, max_edge: u32) -> Option<DynamicImage> {
    let file = File::open(path).ok()?;
    let mut decoder = jpeg_decoder::Decoder::new(BufReader::new(file));
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
fn decode_full(path: &Path) -> Result<DynamicImage, ThumbnailError> {
    let io_error = |source| ThumbnailError::Io {
        path: path.to_path_buf(),
        source,
    };
    ImageReader::open(path)
        .map_err(io_error)?
        .with_guessed_format()
        .map_err(io_error)?
        .decode()
        .map_err(|source| ThumbnailError::Decode {
            path: path.to_path_buf(),
            source,
        })
}

/// Size of a `width` × `height` image scaled down to fit inside
/// `max_edge` × `max_edge`, keeping the aspect ratio. Never scales up.
fn fit_inside(width: u32, height: u32, max_edge: u32) -> (u32, u32) {
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
    fn large_rgb_jpegs_take_the_scaled_decode_path() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("big.jpg");
        RgbImage::from_pixel(1600, 1200, image::Rgb([10, 200, 10]))
            .save(&path)
            .unwrap();

        let image = decode_scaled(&path, 400).unwrap();

        // 1/4 is the smallest DCT scale that still covers 400 × 300.
        assert_eq!((image.width(), image.height()), (400, 300));
    }
}
