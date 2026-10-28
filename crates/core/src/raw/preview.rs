//! Finding the JPEG preview embedded in a RAW file.

use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use super::RawError;
use super::format::RawFormat;
use crate::tiff::{Endian, Ifd, MAX_BLOCK, TiffReader};

/// Bytes of a candidate read to check that it is a displayable JPEG.
const PROBE_LEN: u32 = 256 * 1024;

mod tag {
    pub const COMPRESSION: u16 = 0x0103;
    pub const STRIP_OFFSETS: u16 = 0x0111;
    pub const STRIP_BYTE_COUNTS: u16 = 0x0117;
    pub const JPEG_OFFSET: u16 = 0x0201;
    pub const JPEG_LENGTH: u16 = 0x0202;
    /// Panasonic RW2: a whole JPEG as the value.
    pub const JPG_FROM_RAW: u16 = 0x002E;
    pub const MAKER_NOTE: u16 = 0x927C;
    /// Olympus maker note: the `CameraSettings` IFD.
    pub const OLYMPUS_CAMERA_SETTINGS: u16 = 0x2020;
    pub const OLYMPUS_PREVIEW_START: u16 = 0x0101;
    pub const OLYMPUS_PREVIEW_LENGTH: u16 = 0x0102;
    /// Older Olympus maker notes (offsets from the start of the file).
    pub const OLYMPUS_OLD_PREVIEW_START: u16 = 0x0088;
    pub const OLYMPUS_OLD_PREVIEW_LENGTH: u16 = 0x0089;
}

/// TIFF compression values of JPEG data (old-style and new-style JPEG).
const JPEG_COMPRESSION: [u32; 2] = [6, 7];

/// Where a candidate preview lies in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Candidate {
    offset: u64,
    len: u32,
}

/// Reads the largest displayable JPEG embedded in the RAW file at `path`.
///
/// # Errors
///
/// Fails if the file cannot be read or holds no usable preview.
pub fn read_preview(path: &Path, format: RawFormat) -> Result<Vec<u8>, RawError> {
    let mut file = BufReader::new(File::open(path)?);
    let candidates = if format.is_tiff_based() {
        tiff_candidates(&mut file)
    } else {
        raf_candidates(&mut file)
    };
    let best = candidates
        .into_iter()
        .filter_map(|candidate| Some((candidate, pixels(&mut file, candidate)?)))
        .max_by_key(|&(_, pixels)| pixels)
        .map(|(candidate, _)| candidate)
        .ok_or(RawError::NoPreview)?;
    read_at(&mut file, best.offset, best.len).ok_or(RawError::NoPreview)
}

/// Candidates in the IFDs of a TIFF-structured RAW file and its maker note.
fn tiff_candidates(file: &mut BufReader<File>) -> Vec<Candidate> {
    let Ok((mut tiff, first)) = TiffReader::open(file) else {
        return Vec::new();
    };
    let ifds = tiff.walk(first);
    let mut found: Vec<Candidate> = ifds
        .iter()
        .flat_map(|ifd| ifd_candidates(&mut tiff, ifd))
        .collect();
    if let Some(maker_note) = ifds.iter().find_map(|ifd| ifd.get(tag::MAKER_NOTE))
        && let Some((offset, len)) = tiff.value_location(maker_note)
    {
        found.extend(olympus_candidates(&mut tiff, offset, len));
    }
    found
}

/// Candidates of one IFD: a JPEG thumbnail or preview, JPEG-compressed image
/// data in a single strip, and Panasonic's `JpgFromRaw`.
fn ifd_candidates<R: Read + Seek>(tiff: &mut TiffReader<R>, ifd: &Ifd) -> Vec<Candidate> {
    let mut found = Vec::new();
    let pair = |tiff: &mut TiffReader<R>, offset_tag, length_tag| {
        let offset = tiff.number(ifd.get(offset_tag)?)?;
        let len = tiff.number(ifd.get(length_tag)?)?;
        Some(Candidate {
            offset: u64::from(offset),
            len,
        })
    };
    found.extend(pair(tiff, tag::JPEG_OFFSET, tag::JPEG_LENGTH));
    let jpeg_compressed = ifd
        .get(tag::COMPRESSION)
        .and_then(|entry| tiff.number(entry))
        .is_some_and(|value| JPEG_COMPRESSION.contains(&value));
    let single_strip = ifd
        .get(tag::STRIP_OFFSETS)
        .is_some_and(|entry| entry.count == 1);
    if jpeg_compressed && single_strip {
        found.extend(pair(tiff, tag::STRIP_OFFSETS, tag::STRIP_BYTE_COUNTS));
    }
    if let Some((offset, len)) = ifd
        .get(tag::JPG_FROM_RAW)
        .and_then(|entry| tiff.value_location(entry))
    {
        found.push(Candidate {
            offset: u64::from(offset),
            len,
        });
    }
    found
}

/// The preview in an Olympus maker note at `offset` (`len` bytes long).
fn olympus_candidates<R: Read + Seek>(
    tiff: &mut TiffReader<R>,
    offset: u32,
    len: u32,
) -> Option<Candidate> {
    let header = tiff.bytes(offset, 12.min(len)).ok()?;
    if header.starts_with(b"OLYMPUS\0") {
        // Newer maker notes: own byte order, offsets from the maker note.
        let endian = match header.get(8..10)? {
            b"II" => Endian::Little,
            b"MM" => Endian::Big,
            _ => return None,
        };
        let mut note = tiff.nested(offset, endian);
        let main = note.ifd(12).ok()?;
        let settings_offset = note.number(main.get(tag::OLYMPUS_CAMERA_SETTINGS)?)?;
        let settings = note.ifd(settings_offset).ok()?;
        let start = note.number(settings.get(tag::OLYMPUS_PREVIEW_START)?)?;
        let len = note.number(settings.get(tag::OLYMPUS_PREVIEW_LENGTH)?)?;
        return Some(Candidate {
            offset: u64::from(offset) + u64::from(start),
            len,
        });
    }
    if header.starts_with(b"OLYMP\0") {
        // Older maker notes: the file's byte order and offsets.
        let main = tiff.ifd(offset + 8).ok()?;
        let start = tiff.number(main.get(tag::OLYMPUS_OLD_PREVIEW_START)?)?;
        let len = tiff.number(main.get(tag::OLYMPUS_OLD_PREVIEW_LENGTH)?)?;
        return Some(Candidate {
            offset: u64::from(start),
            len,
        });
    }
    None
}

/// The JPEG a Fujifilm RAF header points to.
fn raf_candidates(file: &mut BufReader<File>) -> Vec<Candidate> {
    let Some(header) = read_at(file, 0, 92) else {
        return Vec::new();
    };
    if !header.starts_with(b"FUJIFILMCCD-RAW") {
        return Vec::new();
    }
    let be = |at: usize| {
        let bytes = header.get(at..at + 4)?;
        Some(u32::from_be_bytes(bytes.try_into().ok()?))
    };
    match (be(84), be(88)) {
        (Some(offset), Some(len)) => vec![Candidate {
            offset: u64::from(offset),
            len,
        }],
        _ => Vec::new(),
    }
}

/// The pixel count of a candidate if it is a JPEG a browser can show
/// (baseline or progressive; RAW data stored as lossless JPEG is not).
fn pixels(file: &mut BufReader<File>, candidate: Candidate) -> Option<u64> {
    if candidate.len < 4 {
        return None;
    }
    let probe = read_at(file, candidate.offset, candidate.len.min(PROBE_LEN))?;
    if !probe.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(probe));
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    let displayable = matches!(
        info.coding_process,
        jpeg_decoder::CodingProcess::DctSequential | jpeg_decoder::CodingProcess::DctProgressive
    );
    displayable.then(|| u64::from(info.width) * u64::from(info.height))
}

/// `len` bytes at `offset`, if they lie inside the file and below the size limit.
fn read_at(file: &mut BufReader<File>, offset: u64, len: u32) -> Option<Vec<u8>> {
    if u64::from(len) > MAX_BLOCK {
        return None;
    }
    let file_len = file.get_ref().metadata().ok()?.len();
    if offset.checked_add(u64::from(len))? > file_len {
        return None;
    }
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut bytes = vec![0; len as usize];
    file.read_exact(&mut bytes).ok()?;
    Some(bytes)
}
