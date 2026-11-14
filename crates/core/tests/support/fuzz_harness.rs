//! What each fuzz target does with one input: call the parsers on it and
//! check what must hold for every input, not only that nothing panics.
//!
//! Shared by the fuzz targets in `fuzz/` (nightly, `cargo fuzz`) and the
//! stable regression tests, so it is compiled on every push and cannot rot.

// A broken invariant is a finding: panicking is what the fuzzer looks for.
#![allow(clippy::expect_used)]

use std::io::Cursor;
use std::path::Path;

use chrono::Datelike;
use sito_core::decode::{ImageSource, JPEG_MEMORY_LIMIT, decode_image, decode_tiff_from};
use sito_core::metadata::{Orientation, PhotoMetadata, read_exif_metadata_from};
use sito_core::raw::{RawFormat, read_preview_from, read_raw_metadata_from, with_orientation};
use sito_core::takeout::parse_sidecar;
use sito_core::tiff::{Endian, MAX_BLOCK, TiffReader, read_tiff_metadata_from};

/// Most IFDs `TiffReader::walk` may return (its own limit).
const MAX_IFDS: usize = 64;
/// Earliest year of a capture time Sito accepts.
const MIN_PLAUSIBLE_YEAR: i32 = 1990;
/// Memory limit of the TIFF decode under fuzzing, far below the app's.
const FUZZ_TIFF_MEMORY_LIMIT: u64 = 64 * 1024 * 1024;
const SOI: [u8; 2] = [0xFF, 0xD8];

/// The TIFF structure reader: headers, IFD chains, values, maker notes.
pub fn tiff_reader(data: &[u8]) {
    let Ok((mut tiff, first)) = TiffReader::open(Cursor::new(data)) else {
        return;
    };
    let ifds = tiff.walk(first);
    assert!(ifds.len() <= MAX_IFDS, "walk returned {} IFDs", ifds.len());
    for entry in ifds.iter().flat_map(|ifd| ifd.entries.iter().copied()) {
        let _ = tiff.numbers(entry);
        let _ = tiff.number(entry);
        let _ = tiff.text(entry);
        let Some((offset, len)) = tiff.value_location(entry) else {
            continue;
        };
        if let Ok(bytes) = tiff.bytes(offset, len) {
            assert_eq!(bytes.len(), len as usize, "bytes at {offset}");
        }
        // A maker note: a structure of its own at the value, either byte order.
        for endian in [Endian::Little, Endian::Big] {
            let mut note = tiff.nested(offset, endian);
            for ifd in note.walk(0).iter().chain(note.walk(8).iter()) {
                for entry in &ifd.entries {
                    let _ = note.numbers(*entry);
                }
            }
        }
    }
}

/// Finding the embedded JPEG of a RAW file, then turning it upright.
pub fn raw_preview(data: &[u8]) {
    for format in [RawFormat::Dng, RawFormat::Raf] {
        let Ok(preview) = read_preview_from(Cursor::new(data), format) else {
            continue;
        };
        assert!(preview.starts_with(&SOI), "a preview is a JPEG");
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            assert_eq!(preview, data, "a JPEG under a RAW name is returned whole");
        } else {
            assert!(preview.len() as u64 <= MAX_BLOCK, "preview over MAX_BLOCK");
        }
        let _ = with_orientation(preview, Orientation::from_exif(6));
    }
}

/// Capture time, orientation and camera, through every reader.
pub fn metadata(data: &[u8]) {
    check_metadata(&read_tiff_metadata_from(Cursor::new(data)));
    check_metadata(&read_exif_metadata_from(Cursor::new(data)));
    check_metadata(&read_raw_metadata_from(Cursor::new(data), RawFormat::Raf));
}

fn check_metadata(meta: &PhotoMetadata) {
    if let Some(captured) = meta.captured {
        assert!(
            captured.local.year() >= MIN_PLAUSIBLE_YEAR,
            "implausible capture time {}",
            captured.local
        );
    }
}

/// Writing an orientation into a JPEG's bytes.
pub fn orientation(data: &[u8]) {
    for value in 1..=8 {
        let out = with_orientation(data.to_vec(), Orientation::from_exif(value));
        if out == data {
            continue;
        }
        assert!(data.starts_with(&SOI), "only a JPEG is changed");
        let inserted = out
            .strip_prefix(&SOI[..])
            .and_then(|rest| rest.strip_suffix(&data[SOI.len()..]))
            .expect("the input with one segment inserted after SOI");
        let segment_len = inserted
            .strip_prefix(&[0xFF, 0xE1][..])
            .and_then(|rest| rest.first_chunk::<2>())
            .map(|len| usize::from(u16::from_be_bytes(*len)))
            .expect("an APP1 segment");
        assert_eq!(segment_len + 2, inserted.len(), "one segment, length right");
        assert_eq!(
            inserted.get(4..10),
            Some(&b"Exif\0\0"[..]),
            "an EXIF segment"
        );
    }
}

/// Takeout date files.
pub fn sidecar(data: &[u8]) {
    let _ = parse_sidecar(data);
}

/// Decoding to pixels: the scaled JPEG decode, the full decode, the TIFF
/// decode with a memory limit.
pub fn decode(data: &[u8]) {
    let source = ImageSource::Bytes {
        bytes: data,
        path: Path::new("fuzz"),
    };
    let scaled = decode_image(source, Some(256));
    let full = decode_image(source, None);
    if claimed_jpeg_bytes(data).is_some_and(|bytes| bytes > JPEG_MEMORY_LIMIT) {
        assert!(
            scaled.is_err() && full.is_err(),
            "a JPEG over the limit is refused"
        );
    }
    let _ = decode_tiff_from(Cursor::new(data), FUZZ_TIFF_MEMORY_LIMIT);
}

/// Bytes the pixels of a JPEG need at full size, as its header claims.
fn claimed_jpeg_bytes(data: &[u8]) -> Option<u64> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(data));
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    let bytes_per_pixel = match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => 1,
        jpeg_decoder::PixelFormat::L16 => 2,
        jpeg_decoder::PixelFormat::RGB24 => 3,
        jpeg_decoder::PixelFormat::CMYK32 => 4,
    };
    Some(u64::from(info.width) * u64::from(info.height) * bytes_per_pixel)
}
