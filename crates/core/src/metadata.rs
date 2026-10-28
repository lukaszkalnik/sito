//! Photo metadata read from EXIF: capture time, orientation and camera.

use std::path::Path;

use chrono::{Datelike, FixedOffset, NaiveDateTime, NaiveTime};
use nom_exif::{EntryValue, Exif, ExifDateTime, ExifTag};

use crate::raw::read_raw_metadata;
use crate::scan::{PhotoKind, photo_kind};
use crate::tiff::read_tiff_metadata;

/// Earliest year accepted as a real capture time; older values are treated
/// as broken (cameras with an unset clock write e.g. `0000:00:00` or 1970).
pub(crate) const MIN_PLAUSIBLE_YEAR: i32 = 1990;

/// How far a capture time may lie after "now" (clocks of camera and computer
/// differ a little); later times are camera clock errors.
const MAX_FUTURE: chrono::TimeDelta = chrono::TimeDelta::days(1);

/// Whether `time` can be a real capture time at `now`.
///
/// Not plausible: years before 1990, exactly January 1 at 00:00:00 (what
/// cameras with a reset clock write) and times more than a day after `now`.
#[must_use]
pub fn is_plausible_capture_time(time: NaiveDateTime, now: NaiveDateTime) -> bool {
    let reset_clock = time.month() == 1 && time.day() == 1 && time.time() == NaiveTime::MIN;
    time.year() >= MIN_PLAUSIBLE_YEAR && !reset_clock && time <= now + MAX_FUTURE
}

/// When a photo was taken, as recorded by the camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureTime {
    /// Wall-clock time on the camera when the photo was taken.
    pub local: NaiveDateTime,
    /// UTC offset of `local`, if the camera recorded one.
    pub offset: Option<FixedOffset>,
}

/// EXIF orientation (1–8). Values outside that range read as the default 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orientation(u16);

impl Orientation {
    /// No rotation or mirroring needed.
    pub const NORMAL: Self = Self(1);

    /// Wraps an EXIF orientation value, falling back to [`Self::NORMAL`].
    #[must_use]
    pub fn from_exif(value: u16) -> Self {
        if (1..=8).contains(&value) {
            Self(value)
        } else {
            Self::NORMAL
        }
    }

    /// The EXIF orientation value (1–8).
    #[must_use]
    pub fn value(self) -> u16 {
        self.0
    }
}

impl Default for Orientation {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Metadata of one photo. Every field is optional in practice: a photo
/// without (readable) EXIF yields the defaults.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PhotoMetadata {
    /// Capture time from EXIF, if present and plausible.
    pub captured: Option<CaptureTime>,
    /// EXIF orientation.
    pub orientation: Orientation,
    /// Camera description, e.g. `Google Pixel 8`.
    pub camera: Option<String>,
}

/// Reads the metadata of the photo at `path`, in the way its kind needs.
///
/// Never fails: files without EXIF, unreadable files and malformed values
/// all yield [`PhotoMetadata::default`] or partially filled metadata.
#[must_use]
pub fn read_photo_metadata(path: &Path) -> PhotoMetadata {
    match photo_kind(path) {
        Some(PhotoKind::Raw(format)) => read_raw_metadata(path, format),
        Some(PhotoKind::Tiff) => read_tiff_metadata(path),
        Some(PhotoKind::Jpeg) | None => read_exif_metadata(path),
    }
}

/// Reads the EXIF metadata of a file `nom-exif` understands (JPEG, RAF, …).
///
/// Never fails, like [`read_photo_metadata`].
#[must_use]
pub fn read_exif_metadata(path: &Path) -> PhotoMetadata {
    match nom_exif::read_exif(path) {
        Ok(exif) => from_exif(&exif),
        Err(_) => PhotoMetadata::default(),
    }
}

fn from_exif(exif: &Exif) -> PhotoMetadata {
    PhotoMetadata {
        captured: capture_time(exif),
        orientation: exif
            .get(ExifTag::Orientation)
            .and_then(EntryValue::as_u16)
            .map(Orientation::from_exif)
            .unwrap_or_default(),
        camera: camera(exif),
    }
}

/// Picks the best available timestamp: when the photo was taken, then when it
/// was digitized, then when the file was last changed by the camera.
fn capture_time(exif: &Exif) -> Option<CaptureTime> {
    [
        ExifTag::DateTimeOriginal,
        ExifTag::CreateDate,
        ExifTag::ModifyDate,
    ]
    .into_iter()
    .filter_map(|tag| exif.get(tag).and_then(EntryValue::as_datetime))
    .map(|value| match value {
        ExifDateTime::Aware(dt) => CaptureTime {
            local: dt.naive_local(),
            offset: Some(*dt.offset()),
        },
        ExifDateTime::Naive(local) => CaptureTime {
            local,
            offset: None,
        },
    })
    .find(|time| time.local.year() >= MIN_PLAUSIBLE_YEAR)
}

fn camera(exif: &Exif) -> Option<String> {
    let text = |tag| {
        exif.get(tag)
            .and_then(EntryValue::as_str)
            .map(|s| s.trim_matches(|c: char| c == '\0' || c.is_whitespace()))
            .filter(|s| !s.is_empty())
    };
    camera_name(text(ExifTag::Make), text(ExifTag::Model))
}

/// A camera description from EXIF `Make` and `Model`, e.g. `Google Pixel 8`.
pub(crate) fn camera_name(make: Option<&str>, model: Option<&str>) -> Option<String> {
    match (make, model) {
        // Many cameras repeat the make in the model ("Canon Canon EOS R6").
        (Some(make), Some(model)) if model.to_lowercase().starts_with(&make.to_lowercase()) => {
            Some(model.to_owned())
        }
        (Some(make), Some(model)) => Some(format!("{make} {model}")),
        (None, Some(model)) => Some(model.to_owned()),
        (Some(make), None) => Some(make.to_owned()),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn plausible_capture_times() {
        let now = NaiveDate::from_ymd_opt(2026, 10, 28)
            .and_then(|d| d.and_hms_opt(12, 0, 0))
            .unwrap();
        let at = |text: &str| NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").unwrap();
        assert!(is_plausible_capture_time(at("2024-06-01 12:00:05"), now));
        assert!(is_plausible_capture_time(at("2025-01-01 00:00:01"), now));
        assert!(is_plausible_capture_time(at("2026-10-29 11:00:00"), now));
        assert!(!is_plausible_capture_time(at("1989-12-31 23:59:59"), now));
        assert!(!is_plausible_capture_time(at("2000-01-01 00:00:00"), now));
        assert!(!is_plausible_capture_time(at("2026-10-29 12:00:01"), now));
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/exif")
            .join(name)
    }

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, mo, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

    #[test]
    fn reads_date_time_original_and_camera() {
        let meta = read_photo_metadata(&fixture("pixel_2024-06-01_12-00-00.jpg"));

        assert_eq!(
            meta.captured,
            Some(CaptureTime {
                local: at(2024, 6, 1, 12, 0, 0),
                offset: None
            })
        );
        assert_eq!(meta.camera.as_deref(), Some("Google Pixel 8"));
        assert_eq!(meta.orientation, Orientation::NORMAL);
    }

    #[test]
    fn keeps_wall_clock_time_and_offset() {
        let meta = read_photo_metadata(&fixture("offset_2024-06-02_08-30-00+0200.jpg"));

        let captured = meta.captured.expect("capture time");
        assert_eq!(captured.local, at(2024, 6, 2, 8, 30, 0));
        assert_eq!(captured.offset, FixedOffset::east_opt(2 * 3600));
        assert_eq!(meta.camera.as_deref(), Some("samsung SM-S911B"));
    }

    #[test]
    fn reads_orientation() {
        let meta = read_photo_metadata(&fixture("rotated_orientation-6.jpg"));

        assert_eq!(meta.orientation.value(), 6);
    }

    #[test]
    fn falls_back_to_modify_date() {
        let meta = read_photo_metadata(&fixture("datetime-only_2024-05-30_10-00-00.jpg"));

        assert_eq!(
            meta.captured.map(|c| c.local),
            Some(at(2024, 5, 30, 10, 0, 0))
        );
        assert_eq!(meta.camera, None);
    }

    #[test]
    fn ignores_broken_dates() {
        let meta = read_photo_metadata(&fixture("broken-date.jpg"));

        assert_eq!(meta.captured, None);
    }

    #[test]
    fn photo_without_exif_has_defaults() {
        assert_eq!(
            read_photo_metadata(&fixture("no-exif.jpg")),
            PhotoMetadata::default()
        );
    }

    #[test]
    fn unreadable_file_has_defaults() {
        assert_eq!(
            read_photo_metadata(&fixture("does-not-exist.jpg")),
            PhotoMetadata::default()
        );
    }

    #[test]
    fn out_of_range_orientation_is_normal() {
        assert_eq!(Orientation::from_exif(0), Orientation::NORMAL);
        assert_eq!(Orientation::from_exif(9), Orientation::NORMAL);
        assert_eq!(Orientation::from_exif(8).value(), 8);
    }
}
