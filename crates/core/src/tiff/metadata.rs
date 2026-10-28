//! Photo metadata (capture time, orientation, camera) from the IFDs of a
//! TIFF-structured file: TIFF images and most RAW formats.

use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use chrono::{Datelike, FixedOffset, NaiveDateTime};

use super::reader::{Ifd, TiffReader};
use crate::metadata::{CaptureTime, MIN_PLAUSIBLE_YEAR, Orientation, PhotoMetadata, camera_name};

mod tag {
    pub const ORIENTATION: u16 = 0x0112;
    pub const MAKE: u16 = 0x010F;
    pub const MODEL: u16 = 0x0110;
    pub const DATE_TIME: u16 = 0x0132;
    pub const DATE_TIME_ORIGINAL: u16 = 0x9003;
    pub const DATE_TIME_DIGITIZED: u16 = 0x9004;
    pub const OFFSET_TIME_ORIGINAL: u16 = 0x9011;
}

/// Reads the metadata of the TIFF-structured file at `path`.
///
/// Never fails: unreadable files and missing or malformed values yield
/// [`PhotoMetadata::default`] or partially filled metadata.
#[must_use]
pub fn read_tiff_metadata(path: &Path) -> PhotoMetadata {
    File::open(path)
        .ok()
        .and_then(|file| from_reader(BufReader::new(file)))
        .unwrap_or_default()
}

/// Reads the metadata of a TIFF structure held in `reader`.
fn from_reader(reader: impl Read + Seek) -> Option<PhotoMetadata> {
    let (mut tiff, first) = TiffReader::open(reader).ok()?;
    let ifds = tiff.walk(first);
    let text = |tiff: &mut TiffReader<_>, tag| {
        ifds.iter()
            .find_map(|ifd| ifd.get(tag).and_then(|entry| tiff.text(entry)))
    };
    let make = text(&mut tiff, tag::MAKE);
    let model = text(&mut tiff, tag::MODEL);
    let orientation = ifds
        .iter()
        .find_map(|ifd| ifd.get(tag::ORIENTATION))
        .and_then(|entry| tiff.number(entry))
        .and_then(|value| u16::try_from(value).ok())
        .map(Orientation::from_exif)
        .unwrap_or_default();
    Some(PhotoMetadata {
        captured: capture_time(&mut tiff, &ifds),
        orientation,
        camera: camera_name(make.as_deref(), model.as_deref()),
    })
}

/// The first plausible of `DateTimeOriginal`, `DateTimeDigitized` and
/// `DateTime`, like the EXIF of a JPEG.
fn capture_time<R: Read + Seek>(tiff: &mut TiffReader<R>, ifds: &[Ifd]) -> Option<CaptureTime> {
    let offset = ifds
        .iter()
        .find_map(|ifd| ifd.get(tag::OFFSET_TIME_ORIGINAL))
        .and_then(|entry| tiff.text(entry))
        .and_then(|text| parse_offset(&text));
    [
        tag::DATE_TIME_ORIGINAL,
        tag::DATE_TIME_DIGITIZED,
        tag::DATE_TIME,
    ]
    .into_iter()
    .filter_map(|tag| {
        let text = ifds
            .iter()
            .find_map(|ifd| ifd.get(tag).and_then(|entry| tiff.text(entry)))?;
        Some((tag, parse_date_time(&text)?))
    })
    .find(|(_, local)| local.year() >= MIN_PLAUSIBLE_YEAR)
    .map(|(tag, local)| CaptureTime {
        local,
        // The offset belongs to the original capture time only.
        offset: offset.filter(|_| tag == tag::DATE_TIME_ORIGINAL),
    })
}

/// `2024:06:01 12:00:05` → that local time.
fn parse_date_time(text: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(text.trim(), "%Y:%m:%d %H:%M:%S").ok()
}

/// `+02:00` → that offset.
fn parse_offset(text: &str) -> Option<FixedOffset> {
    let text = text.trim();
    let (sign, rest) = match text.as_bytes().first()? {
        b'+' => (1, &text[1..]),
        b'-' => (-1, &text[1..]),
        _ => return None,
    };
    let (hours, minutes) = rest.split_once(':')?;
    let seconds = hours.parse::<i32>().ok()? * 3600 + minutes.parse::<i32>().ok()? * 60;
    FixedOffset::east_opt(sign * seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exif_dates_and_offsets() {
        let local = parse_date_time("2024:06:01 12:00:05").unwrap();
        assert_eq!(local.to_string(), "2024-06-01 12:00:05");
        assert_eq!(parse_date_time("0000:00:00 00:00:00"), None);
        assert_eq!(parse_offset("+02:00"), FixedOffset::east_opt(7200));
        assert_eq!(parse_offset("-05:30"), FixedOffset::west_opt(19800));
        assert_eq!(parse_offset("02:00"), None);
    }
}
