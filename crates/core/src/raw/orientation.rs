//! Giving an embedded RAW preview the RAW file's orientation.

use crate::metadata::Orientation;

const SOI: [u8; 2] = [0xFF, 0xD8];
const APP1: u8 = 0xE1;

/// `jpeg` with an EXIF segment holding `orientation`, so viewers turn it
/// upright like a camera JPEG. RAW previews usually have no EXIF of their
/// own; a preview that already has an EXIF segment, or an upright
/// orientation, is returned unchanged.
#[must_use]
pub fn with_orientation(jpeg: Vec<u8>, orientation: Orientation) -> Vec<u8> {
    if orientation == Orientation::NORMAL || !jpeg.starts_with(&SOI) || has_exif(&jpeg) {
        return jpeg;
    }
    let mut out = Vec::with_capacity(jpeg.len() + 40);
    out.extend_from_slice(&SOI);
    out.extend_from_slice(&exif_segment(orientation));
    out.extend_from_slice(&jpeg[2..]);
    out
}

/// An APP1 segment with a minimal big-endian EXIF block: IFD0 with only
/// the Orientation tag.
fn exif_segment(orientation: Orientation) -> Vec<u8> {
    let mut tiff = Vec::new();
    tiff.extend_from_slice(b"MM\0\x2A\0\0\0\x08");
    tiff.extend_from_slice(&1u16.to_be_bytes());
    tiff.extend_from_slice(&0x0112u16.to_be_bytes());
    tiff.extend_from_slice(&3u16.to_be_bytes());
    tiff.extend_from_slice(&1u32.to_be_bytes());
    tiff.extend_from_slice(&orientation.value().to_be_bytes());
    tiff.extend_from_slice(&[0, 0]);
    tiff.extend_from_slice(&0u32.to_be_bytes());

    let payload_len = 6 + tiff.len();
    let segment_len = u16::try_from(payload_len + 2).unwrap_or(u16::MAX);
    let mut segment = vec![0xFF, APP1];
    segment.extend_from_slice(&segment_len.to_be_bytes());
    segment.extend_from_slice(b"Exif\0\0");
    segment.extend_from_slice(&tiff);
    segment
}

/// Whether one of the segments before the image data is an EXIF APP1.
fn has_exif(jpeg: &[u8]) -> bool {
    let mut at = 2;
    while let Some(&[0xFF, marker, high, low]) = jpeg.get(at..at + 4) {
        // Start of scan: no more header segments.
        if marker == 0xDA {
            return false;
        }
        if marker == APP1 && jpeg.get(at + 4..at + 10) == Some(b"Exif\0\0") {
            return true;
        }
        at += 2 + usize::from(u16::from_be_bytes([high, low]));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOI, an APP0 segment and the start of scan.
    const BARE: &[u8] = b"\xFF\xD8\xFF\xE0\x00\x04ab\xFF\xDA\x00\x02";

    #[test]
    fn adds_the_orientation_to_a_preview_without_exif() {
        let out = with_orientation(BARE.to_vec(), Orientation::from_exif(6));

        assert!(has_exif(&out));
        assert!(out.starts_with(b"\xFF\xD8\xFF\xE1"));
        assert!(out.ends_with(&BARE[2..]));
        // Big-endian SHORT value 6 inside the IFD entry.
        assert!(out.windows(4).any(|w| w == [0x00, 0x06, 0x00, 0x00]));
    }

    #[test]
    fn leaves_upright_previews_and_ones_with_exif_alone() {
        assert_eq!(with_orientation(BARE.to_vec(), Orientation::NORMAL), BARE);
        let with_exif = with_orientation(BARE.to_vec(), Orientation::from_exif(8));
        assert_eq!(
            with_orientation(with_exif.clone(), Orientation::from_exif(6)),
            with_exif
        );
    }
}
