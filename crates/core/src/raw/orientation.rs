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
    let Some(image) = jpeg.get(SOI.len()..) else {
        return jpeg;
    };
    let segment = exif_segment(orientation);
    let mut out = Vec::with_capacity(jpeg.len().saturating_add(segment.len()));
    out.extend_from_slice(&SOI);
    out.extend_from_slice(&segment);
    out.extend_from_slice(image);
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

    let mut payload = b"Exif\0\0".to_vec();
    payload.extend_from_slice(&tiff);
    // The length counts its own two bytes.
    let segment_len = u16::try_from(payload.len().saturating_add(2)).unwrap_or(u16::MAX);
    let mut segment = vec![0xFF, APP1];
    segment.extend_from_slice(&segment_len.to_be_bytes());
    segment.extend_from_slice(&payload);
    segment
}

/// Whether one of the segments before the image data is an EXIF APP1.
fn has_exif(jpeg: &[u8]) -> bool {
    let mut rest = jpeg.get(SOI.len()..).unwrap_or_default();
    while let Some((&[0xFF, marker, high, low], body)) = rest.split_first_chunk::<4>() {
        // Start of scan: no more header segments.
        if marker == 0xDA {
            return false;
        }
        if marker == APP1 && body.starts_with(b"Exif\0\0") {
            return true;
        }
        // The length counts its own two bytes, which are already taken.
        let body_len = usize::from(u16::from_be_bytes([high, low])).saturating_sub(2);
        rest = body.get(body_len..).unwrap_or_default();
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

    #[test]
    fn broken_segments_end_the_search_without_exif() {
        let turned = Orientation::from_exif(6);
        // A segment length of 0 (less than its own two bytes), a segment
        // longer than the file, a file that ends inside a segment header.
        let broken: [&[u8]; 4] = [
            b"\xFF\xD8\xFF\xE0\x00\x00\xFF\xE0\x00\x00",
            b"\xFF\xD8\xFF\xE0\xFF\xFFab",
            b"\xFF\xD8\xFF\xE1\x00",
            b"\xFF\xD8",
        ];
        for jpeg in broken {
            assert!(!has_exif(jpeg));
            assert!(has_exif(&with_orientation(jpeg.to_vec(), turned)));
        }
    }
}
