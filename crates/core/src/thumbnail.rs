//! Small, upright JPEG previews of photos.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, GrayImage, ImageError, ImageReader, RgbImage};

use crate::metadata::Orientation;
use crate::raw::{RawError, read_preview};
use crate::scan::{PhotoKind, photo_kind};
use crate::timeline::Photo;

/// Default longest edge of a thumbnail in pixels: covers a 160×120 tile on a
/// 2× display.
pub const DEFAULT_MAX_EDGE: u32 = 400;

/// JPEG quality of thumbnails (1–100).
const JPEG_QUALITY: u8 = 80;

/// Part of every cache key. Bump it when thumbnails are rendered differently,
/// so old cache files are no longer used.
const FORMAT_VERSION: u32 = 1;

/// Thumbnails stored on disk, keyed by photo path, size and modification
/// time, so a changed photo gets a new thumbnail.
#[derive(Debug, Clone)]
pub struct ThumbnailCache {
    dir: PathBuf,
    max_edge: u32,
}

impl ThumbnailCache {
    /// A cache in `dir` (created on first write) with [`DEFAULT_MAX_EDGE`].
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            max_edge: DEFAULT_MAX_EDGE,
        }
    }

    /// Folder holding the cached thumbnails.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Where the thumbnail of `photo` is (or will be) stored. Files are spread
    /// over 256 subfolders so none of them grows huge.
    #[must_use]
    pub fn path_for(&self, photo: &Photo) -> PathBuf {
        let modified_nanos = photo
            .modified
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        let mut hasher = blake3::Hasher::new();
        hasher.update(&FORMAT_VERSION.to_le_bytes());
        hasher.update(&self.max_edge.to_le_bytes());
        hasher.update(&photo.size.to_le_bytes());
        hasher.update(&modified_nanos.to_le_bytes());
        hasher.update(photo.path.as_os_str().as_encoded_bytes());
        let hex = hasher.finalize().to_hex();
        self.dir.join(&hex[..2]).join(format!("{hex}.jpg"))
    }

    /// Returns the cached thumbnail of `photo`, rendering and storing it first
    /// if needed. Safe to call from several threads for the same photo.
    ///
    /// # Errors
    ///
    /// Returns [`ThumbnailError`] if the cached file cannot be read or the
    /// photo cannot be rendered. Failing to store a new thumbnail is not an
    /// error: it is returned anyway and rendered again next time.
    pub fn get_or_create(&self, photo: &Photo) -> Result<Vec<u8>, ThumbnailError> {
        let path = self.path_for(photo);
        match fs::read(&path) {
            Ok(bytes) => return Ok(bytes),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(ThumbnailError::Io { path, source }),
        }

        let bytes = render_photo(&photo.path, photo.orientation, self.max_edge)?;
        // Best effort, see above: a full disk must not hide the thumbnail.
        let _ = store_atomically(&path, &bytes);
        Ok(bytes)
    }

    /// Removes thumbnails no photo in `needed` uses any more (of photos that
    /// were edited, moved or deleted), then, while the cache is larger than
    /// `max_bytes`, the oldest ones. Only files named like cached thumbnails
    /// are touched; a removed thumbnail is simply rendered again when needed.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache folder cannot be listed. Files that
    /// cannot be removed are skipped.
    pub fn clean(&self, needed: &[Photo], max_bytes: u64) -> io::Result<CleanReport> {
        let needed: HashSet<PathBuf> = needed.iter().map(|photo| self.path_for(photo)).collect();
        let mut report = CleanReport::default();
        let mut kept = Vec::new();
        let subfolders = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(report),
            Err(err) => return Err(err),
        };
        for subfolder in subfolders.flatten() {
            let Ok(files) = fs::read_dir(subfolder.path()) else {
                continue;
            };
            for file in files.flatten() {
                let path = file.path();
                if !is_thumbnail_file(&path) {
                    continue;
                }
                let Ok(meta) = file.metadata() else {
                    continue;
                };
                if !needed.contains(&path) {
                    if fs::remove_file(&path).is_ok() {
                        report.orphans_removed += 1;
                    }
                    continue;
                }
                let age = meta.modified().unwrap_or(UNIX_EPOCH);
                kept.push((age, meta.len(), path));
            }
        }

        let mut total: u64 = kept.iter().map(|(_, len, _)| len).sum();
        kept.sort();
        for (_, len, path) in kept {
            if total <= max_bytes {
                break;
            }
            if fs::remove_file(&path).is_ok() {
                total -= len;
                report.oldest_removed += 1;
            }
        }
        report.bytes_left = total;
        Ok(report)
    }
}

/// What [`ThumbnailCache::clean`] removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CleanReport {
    /// Thumbnails of photos that are no longer known.
    pub orphans_removed: usize,
    /// Thumbnails removed, oldest first, to get under the size limit.
    pub oldest_removed: usize,
    /// Size of the thumbnails left, in bytes.
    pub bytes_left: u64,
}

/// Whether `path` is named like a cached thumbnail: `<2 hex>/<64 hex>.jpg`.
fn is_thumbnail_file(path: &Path) -> bool {
    let is_hex = |s: &str, len: usize| s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit());
    let name_ok = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".jpg"))
        .is_some_and(|stem| is_hex(stem, 64));
    let folder_ok = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|n| n.to_str())
        .is_some_and(|n| is_hex(n, 2));
    name_ok && folder_ok
}

/// Writes `bytes` to a temporary file next to `path` and renames it into
/// place, so readers never see a half-written thumbnail.
fn store_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension(format!(
        "{}-{}.tmp",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&temp, bytes)?;
    if let Err(err) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        // Another thread may have stored the same thumbnail first (on Windows
        // replacing a file that is being read fails); that is fine.
        if !path.exists() {
            return Err(err);
        }
    }
    Ok(())
}

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
    /// A RAW photo's embedded preview could not be read.
    #[error("cannot read the preview of {path}")]
    Raw {
        /// Photo that failed.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: RawError,
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

/// Where the image to render comes from.
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
    render_image(ImageSource::Path(path), orientation, max_edge)
}

/// Like [`render_thumbnail`], for a photo of any [`PhotoKind`]: a RAW file
/// is rendered from its embedded preview.
///
/// # Errors
///
/// Returns [`ThumbnailError`] if the photo cannot be read or decoded, or a
/// RAW file has no usable preview.
pub fn render_photo(
    path: &Path,
    orientation: Orientation,
    max_edge: u32,
) -> Result<Vec<u8>, ThumbnailError> {
    match photo_kind(path) {
        Some(PhotoKind::Raw(format)) => {
            let preview = read_preview(path, format).map_err(|source| ThumbnailError::Raw {
                path: path.to_path_buf(),
                source,
            })?;
            let source = ImageSource::Bytes {
                bytes: &preview,
                path,
            };
            render_image(source, orientation, max_edge)
        }
        Some(PhotoKind::Jpeg) | None => render_thumbnail(path, orientation, max_edge),
    }
}

/// Like [`render_thumbnail`], for an image from any [`ImageSource`].
///
/// # Errors
///
/// Returns [`ThumbnailError`] if the image cannot be read or decoded.
pub fn render_image(
    source: ImageSource<'_>,
    orientation: Orientation,
    max_edge: u32,
) -> Result<Vec<u8>, ThumbnailError> {
    let path = source.path();
    let image = match decode_scaled(source, max_edge) {
        Some(image) => image,
        None => decode_full(source)?,
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
fn decode_full(source: ImageSource<'_>) -> Result<DynamicImage, ThumbnailError> {
    let path = source.path();
    let io_error = |source| ThumbnailError::Io {
        path: path.to_path_buf(),
        source,
    };
    let decode_error = |source| ThumbnailError::Decode {
        path: path.to_path_buf(),
        source,
    };
    match source {
        ImageSource::Path(path) => ImageReader::open(path)
            .map_err(io_error)?
            .with_guessed_format()
            .map_err(io_error)?
            .decode()
            .map_err(decode_error),
        ImageSource::Bytes { bytes, .. } => ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(io_error)?
            .decode()
            .map_err(decode_error),
    }
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

        let image = decode_scaled(ImageSource::Path(&path), 400).unwrap();

        // 1/4 is the smallest DCT scale that still covers 400 × 300.
        assert_eq!((image.width(), image.height()), (400, 300));
    }
}
