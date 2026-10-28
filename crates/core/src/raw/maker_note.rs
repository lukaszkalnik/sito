//! Previews hidden in camera makers' own data (the EXIF maker note), whose
//! layout differs per maker.

use std::io::{Read, Seek};

use super::preview::Candidate;
use crate::tiff::{Endian, TiffReader};

mod tag {
    /// Olympus: the `CameraSettings` IFD.
    pub const OLYMPUS_CAMERA_SETTINGS: u16 = 0x2020;
    pub const OLYMPUS_PREVIEW_START: u16 = 0x0101;
    pub const OLYMPUS_PREVIEW_LENGTH: u16 = 0x0102;
    /// Older Olympus maker notes (offsets from the start of the file).
    pub const OLYMPUS_OLD_PREVIEW_START: u16 = 0x0088;
    pub const OLYMPUS_OLD_PREVIEW_LENGTH: u16 = 0x0089;
    /// Older Olympus maker notes: a small JPEG as the value.
    pub const OLYMPUS_OLD_THUMBNAIL: u16 = 0x0100;
    /// Nikon: an IFD describing the preview.
    pub const NIKON_PREVIEW_IFD: u16 = 0x0011;
    pub const JPEG_OFFSET: u16 = 0x0201;
    pub const JPEG_LENGTH: u16 = 0x0202;
}

/// Preview candidates in the maker note at `offset` (`len` bytes long).
pub fn candidates<R: Read + Seek>(
    tiff: &mut TiffReader<R>,
    offset: u32,
    len: u32,
) -> Vec<Candidate> {
    let Ok(header) = tiff.bytes(offset, 18.min(len)) else {
        return Vec::new();
    };
    if header.starts_with(b"OLYMPUS\0") {
        olympus(tiff, offset, &header).into_iter().collect()
    } else if header.starts_with(b"OLYMP\0") {
        olympus_old(tiff, offset)
    } else if header.starts_with(b"Nikon\0") {
        nikon(tiff, offset, &header).into_iter().collect()
    } else {
        Vec::new()
    }
}

/// Newer Olympus maker notes: their own byte order, offsets counted from
/// the maker note; the preview is described in `CameraSettings`.
fn olympus<R: Read + Seek>(
    tiff: &mut TiffReader<R>,
    offset: u32,
    header: &[u8],
) -> Option<Candidate> {
    let endian = endian_of(header.get(8..10)?)?;
    let mut note = tiff.nested(offset, endian);
    let main = note.ifd(12).ok()?;
    let settings_offset = note.number(main.get(tag::OLYMPUS_CAMERA_SETTINGS)?)?;
    let settings = note.ifd(settings_offset).ok()?;
    let start = note.number(settings.get(tag::OLYMPUS_PREVIEW_START)?)?;
    let len = note.number(settings.get(tag::OLYMPUS_PREVIEW_LENGTH)?)?;
    Some(Candidate::at(u64::from(offset) + u64::from(start), len))
}

/// Older Olympus maker notes: the file's byte order and offsets; a preview
/// by start and length, or a small thumbnail as a value.
fn olympus_old<R: Read + Seek>(tiff: &mut TiffReader<R>, offset: u32) -> Vec<Candidate> {
    let Ok(main) = tiff.ifd(offset + 8) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    if let (Some(start), Some(len)) = (
        main.get(tag::OLYMPUS_OLD_PREVIEW_START)
            .and_then(|e| tiff.number(e)),
        main.get(tag::OLYMPUS_OLD_PREVIEW_LENGTH)
            .and_then(|e| tiff.number(e)),
    ) {
        found.push(Candidate::at(u64::from(start), len));
    }
    if let Some((start, len)) = main
        .get(tag::OLYMPUS_OLD_THUMBNAIL)
        .and_then(|entry| tiff.value_location(entry))
    {
        found.push(Candidate::at(u64::from(start), len));
    }
    found
}

/// Nikon maker notes: `Nikon\0`, a version, then a TIFF structure of their
/// own (offsets counted from its header) whose preview IFD holds the JPEG.
fn nikon<R: Read + Seek>(
    tiff: &mut TiffReader<R>,
    offset: u32,
    header: &[u8],
) -> Option<Candidate> {
    let endian = endian_of(header.get(10..12)?)?;
    let base = offset + 10;
    let mut note = tiff.nested(base, endian);
    let first = u32::from_be_bytes(header.get(14..18)?.try_into().ok()?);
    let first = match endian {
        Endian::Big => first,
        Endian::Little => first.swap_bytes(),
    };
    let main = note.ifd(first).ok()?;
    let preview_offset = note.number(main.get(tag::NIKON_PREVIEW_IFD)?)?;
    let preview = note.ifd(preview_offset).ok()?;
    let start = note.number(preview.get(tag::JPEG_OFFSET)?)?;
    let len = note.number(preview.get(tag::JPEG_LENGTH)?)?;
    Some(Candidate::at(u64::from(base) + u64::from(start), len))
}

fn endian_of(mark: &[u8]) -> Option<Endian> {
    match mark {
        b"II" => Some(Endian::Little),
        b"MM" => Some(Endian::Big),
        _ => None,
    }
}
