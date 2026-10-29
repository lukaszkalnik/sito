//! Small, upright JPEG previews of photos.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageError};

pub use crate::decode::ImageSource;
use crate::decode::{DecodeError, decode_image, decode_photo};
use crate::metadata::Orientation;
use crate::raw::{RawError, with_orientation};
use crate::timeline::Photo;

/// Default longest edge of a thumbnail in pixels: covers a 160×120 tile on a
/// 2× display.
pub const DEFAULT_MAX_EDGE: u32 = 400;

/// JPEG quality of thumbnails (1–100).
const JPEG_QUALITY: u8 = 80;

/// JPEG quality of display images (1–100).
const DISPLAY_JPEG_QUALITY: u8 = 90;

/// Longest edge of a display image in pixels: sharp on any screen, while a
/// large scan's JPEG stays a few MB instead of 10–18 MB in the cache.
pub const DISPLAY_MAX_EDGE: u32 = 4096;

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
        self.key_path(photo, self.max_edge)
    }

    /// Where the full-size display image of `photo` is (or will be) stored.
    #[must_use]
    pub fn display_path_for(&self, photo: &Photo) -> PathBuf {
        self.key_path(photo, DISPLAY_MAX_EDGE)
    }

    fn key_path(&self, photo: &Photo, edge: u32) -> PathBuf {
        let modified_nanos = photo
            .modified
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        let mut hasher = blake3::Hasher::new();
        hasher.update(&FORMAT_VERSION.to_le_bytes());
        hasher.update(&edge.to_le_bytes());
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

    /// Returns a JPEG of `photo` for viewing, at most [`DISPLAY_MAX_EDGE`]
    /// pixels on its longest edge, converting and storing it first if
    /// needed: for photos a browser cannot show (TIFF).
    /// It carries the photo's orientation, so the viewer turns it upright.
    /// It lives next to the thumbnails and counts in the same size limit.
    ///
    /// # Errors
    ///
    /// Returns [`ThumbnailError`] if the cached file cannot be read or the
    /// photo cannot be decoded or encoded.
    pub fn display_jpeg_for(&self, photo: &Photo) -> Result<Vec<u8>, ThumbnailError> {
        let path = self.display_path_for(photo);
        match fs::read(&path) {
            Ok(bytes) => return Ok(bytes),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(ThumbnailError::Io { path, source }),
        }

        let image = decode_photo(&photo.path, None)?;
        let image = if image.width().max(image.height()) > DISPLAY_MAX_EDGE {
            image.resize(DISPLAY_MAX_EDGE, DISPLAY_MAX_EDGE, FilterType::Lanczos3)
        } else {
            image
        };
        let mut bytes = Vec::new();
        JpegEncoder::new_with_quality(Cursor::new(&mut bytes), DISPLAY_JPEG_QUALITY)
            .encode_image(&image.into_rgb8())
            .map_err(|source| ThumbnailError::Encode {
                path: photo.path.clone(),
                source,
            })?;
        let bytes = with_orientation(bytes, photo.orientation);
        // Best effort, like thumbnails.
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
        let needed: HashSet<PathBuf> = needed
            .iter()
            .flat_map(|photo| [self.path_for(photo), self.display_path_for(photo)])
            .collect();
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

impl From<DecodeError> for ThumbnailError {
    fn from(err: DecodeError) -> Self {
        match err {
            DecodeError::Io { path, source } => Self::Io { path, source },
            DecodeError::Image { path, source } => Self::Decode { path, source },
            DecodeError::Raw { path, source } => Self::Raw { path, source },
        }
    }
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

/// Like [`render_thumbnail`], for a photo of any kind: a RAW file is
/// rendered from its embedded preview.
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
    let image = decode_photo(path, Some(max_edge))?;
    encode_thumbnail(image, path, orientation, max_edge)
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
    let image = decode_image(source, Some(max_edge))?;
    encode_thumbnail(image, source.path(), orientation, max_edge)
}

/// Shrinks `image` to fit `max_edge`, turns it upright and encodes it.
fn encode_thumbnail(
    image: DynamicImage,
    path: &Path,
    orientation: Orientation,
    max_edge: u32,
) -> Result<Vec<u8>, ThumbnailError> {
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
