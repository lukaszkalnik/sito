//! Reading TIFF structures (headers, IFDs, values) from a file, reading only
//! the bytes needed and refusing anything out of range.

use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom};

/// Most entries accepted in one IFD; real files have a few dozen.
const MAX_ENTRIES: u16 = 1024;
/// Most IFDs followed in one file (chain, sub-IFDs, Exif IFD).
const MAX_IFDS: usize = 64;
/// Most values read for one entry (e.g. strip offsets).
const MAX_VALUES: u32 = 4096;
/// Longest text value read.
const MAX_TEXT: u32 = 1024;
/// Largest block of bytes read at once (an embedded preview).
pub const MAX_BLOCK: u64 = 64 * 1024 * 1024;

/// IFD tags the walker follows into further IFDs.
pub mod tag {
    /// Sub-IFDs (full-size images, previews).
    pub const SUB_IFDS: u16 = 0x014A;
    /// The Exif IFD.
    pub const EXIF_IFD: u16 = 0x8769;
}

/// Byte order of a TIFF structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    /// `II`: least significant byte first.
    Little,
    /// `MM`: most significant byte first.
    Big,
}

impl Endian {
    fn u16(self, bytes: [u8; 2]) -> u16 {
        match self {
            Self::Little => u16::from_le_bytes(bytes),
            Self::Big => u16::from_be_bytes(bytes),
        }
    }

    fn u32(self, bytes: [u8; 4]) -> u32 {
        match self {
            Self::Little => u32::from_le_bytes(bytes),
            Self::Big => u32::from_be_bytes(bytes),
        }
    }
}

/// Why a TIFF structure could not be read.
#[derive(Debug, thiserror::Error)]
pub enum TiffError {
    /// The file could not be read.
    #[error("cannot read the file")]
    Io(#[from] std::io::Error),
    /// The data is not a TIFF structure or points outside the file.
    #[error("not a valid TIFF structure")]
    Invalid,
}

/// One IFD entry: a tag with a typed value, stored inline or at an offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The tag number.
    pub tag: u16,
    /// The TIFF field type (3 = SHORT, 4 = LONG, 2 = ASCII, 7 = UNDEFINED, …).
    pub kind: u16,
    /// Number of values.
    pub count: u32,
    /// The value field as stored: the value itself if it fits in 4 bytes,
    /// else the offset of the value.
    pub field: [u8; 4],
}

impl Entry {
    /// Size in bytes of one value of this entry's type, 0 for unknown types.
    fn value_size(self) -> u32 {
        match self.kind {
            1 | 2 | 6 | 7 => 1,
            3 | 8 => 2,
            4 | 9 | 11 | 13 => 4,
            5 | 10 | 12 => 8,
            _ => 0,
        }
    }

    /// Total size of the value in bytes, if known and not absurd.
    fn byte_len(self) -> Option<u32> {
        self.count
            .checked_mul(self.value_size())
            .filter(|&len| len > 0)
    }
}

/// One IFD: its entries and the offset of the next IFD in the chain (0 for none).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ifd {
    /// The entries, in file order.
    pub entries: Vec<Entry>,
    /// Offset of the next IFD, 0 at the end of the chain.
    pub next: u32,
}

impl Ifd {
    /// The entry with `tag`, if present.
    #[must_use]
    pub fn get(&self, tag: u16) -> Option<Entry> {
        self.entries.iter().copied().find(|entry| entry.tag == tag)
    }
}

/// A TIFF structure inside a file: offsets are relative to `base`.
#[derive(Debug)]
pub struct TiffReader<R> {
    reader: R,
    endian: Endian,
    base: u64,
    file_len: u64,
}

impl<R: Read + Seek> TiffReader<R> {
    /// Reads the TIFF header at the start of `reader`. Any magic number is
    /// accepted after the byte order mark, since RAW formats change it
    /// (`IIRO` for Olympus, `IIU\0` for Panasonic). Returns the reader and
    /// the offset of the first IFD.
    ///
    /// # Errors
    ///
    /// Fails if the data does not start with `II` or `MM`.
    pub fn open(reader: R) -> Result<(Self, u32), TiffError> {
        Self::open_at(reader, 0)
    }

    /// Like [`Self::open`], for a TIFF structure that starts at `base`.
    ///
    /// # Errors
    ///
    /// Fails if the data at `base` does not start with `II` or `MM`.
    pub fn open_at(mut reader: R, base: u64) -> Result<(Self, u32), TiffError> {
        let file_len = reader.seek(SeekFrom::End(0))?;
        let mut tiff = Self {
            reader,
            endian: Endian::Little,
            base,
            file_len,
        };
        let header: [u8; 8] = tiff.array(0)?;
        tiff.endian = match &header[..2] {
            b"II" => Endian::Little,
            b"MM" => Endian::Big,
            _ => return Err(TiffError::Invalid),
        };
        let first = tiff
            .endian
            .u32([header[4], header[5], header[6], header[7]]);
        Ok((tiff, first))
    }

    /// A reader for a structure at `base` (in this reader's offsets) with the
    /// given byte order, e.g. a maker note with offsets of its own.
    pub fn nested(&mut self, base: u32, endian: Endian) -> TiffReader<&mut R> {
        TiffReader {
            base: self.base + u64::from(base),
            endian,
            file_len: self.file_len,
            reader: &mut self.reader,
        }
    }

    /// The byte order.
    #[must_use]
    pub fn endian(&self) -> Endian {
        self.endian
    }

    /// Reads the IFD at `offset`.
    ///
    /// # Errors
    ///
    /// Fails if the IFD lies outside the file or has an absurd entry count.
    pub fn ifd(&mut self, offset: u32) -> Result<Ifd, TiffError> {
        let count = self.endian.u16(self.array(offset)?);
        if count == 0 || count > MAX_ENTRIES {
            return Err(TiffError::Invalid);
        }
        let bytes = self.bytes(offset + 2, u32::from(count) * 12 + 4)?;
        let entries = bytes
            .as_chunks::<12>()
            .0
            .iter()
            .map(|raw| Entry {
                tag: self.endian.u16([raw[0], raw[1]]),
                kind: self.endian.u16([raw[2], raw[3]]),
                count: self.endian.u32([raw[4], raw[5], raw[6], raw[7]]),
                field: [raw[8], raw[9], raw[10], raw[11]],
            })
            .collect();
        let tail = &bytes[bytes.len() - 4..];
        let next = self.endian.u32([tail[0], tail[1], tail[2], tail[3]]);
        Ok(Ifd { entries, next })
    }

    /// Every IFD reachable from `first`: the IFD chain, sub-IFDs and the Exif
    /// IFD, each once, at most a fixed number. Unreadable IFDs are skipped.
    pub fn walk(&mut self, first: u32) -> Vec<Ifd> {
        let mut seen = HashSet::new();
        let mut pending = vec![first];
        let mut found = Vec::new();
        while let Some(offset) = pending.pop() {
            if offset == 0 || !seen.insert(offset) || found.len() >= MAX_IFDS {
                continue;
            }
            let Ok(ifd) = self.ifd(offset) else {
                continue;
            };
            pending.push(ifd.next);
            for tag in [tag::SUB_IFDS, tag::EXIF_IFD] {
                if let Some(entry) = ifd.get(tag) {
                    pending.extend(self.numbers(entry).unwrap_or_default());
                }
            }
            found.push(ifd);
        }
        found
    }

    /// The first value of an integer entry (BYTE, SHORT or LONG, or an IFD
    /// pointer), if it has one.
    #[must_use]
    pub fn number(&mut self, entry: Entry) -> Option<u32> {
        self.numbers(entry).ok()?.into_iter().next()
    }

    /// All values of an integer entry (BYTE, SHORT, LONG or IFD pointers).
    ///
    /// # Errors
    ///
    /// Fails if the entry is not an integer type or its values lie outside the file.
    pub fn numbers(&mut self, entry: Entry) -> Result<Vec<u32>, TiffError> {
        let size = match entry.kind {
            1 => 1,
            3 => 2,
            4 | 13 => 4,
            _ => return Err(TiffError::Invalid),
        };
        if entry.count == 0 || entry.count > MAX_VALUES {
            return Err(TiffError::Invalid);
        }
        let bytes = self.value_bytes(entry)?;
        Ok(bytes
            .chunks_exact(size)
            .map(|v| match v {
                [b] => u32::from(*b),
                [a, b] => u32::from(self.endian.u16([*a, *b])),
                [a, b, c, d] => self.endian.u32([*a, *b, *c, *d]),
                _ => 0,
            })
            .collect())
    }

    /// The text of an ASCII entry, without trailing NULs and spaces.
    #[must_use]
    pub fn text(&mut self, entry: Entry) -> Option<String> {
        if entry.kind != 2 || entry.count == 0 || entry.count > MAX_TEXT {
            return None;
        }
        let bytes = self.value_bytes(entry).ok()?;
        let text = String::from_utf8_lossy(&bytes);
        let text = text.trim_end_matches(|c: char| c == '\0' || c.is_whitespace());
        Some(text.to_owned()).filter(|t| !t.is_empty())
    }

    /// Where an entry's value lies, as `(offset, length)`; inline values have
    /// no offset of their own and give `None`.
    #[must_use]
    pub fn value_location(&self, entry: Entry) -> Option<(u32, u32)> {
        let len = entry.byte_len()?;
        (len > 4).then(|| (self.endian.u32(entry.field), len))
    }

    /// `len` bytes at `offset`.
    ///
    /// # Errors
    ///
    /// Fails if the range lies outside the file or exceeds [`MAX_BLOCK`].
    pub fn bytes(&mut self, offset: u32, len: u32) -> Result<Vec<u8>, TiffError> {
        let start = self.base + u64::from(offset);
        let end = start + u64::from(len);
        if u64::from(len) > MAX_BLOCK || end > self.file_len {
            return Err(TiffError::Invalid);
        }
        self.reader.seek(SeekFrom::Start(start))?;
        let mut bytes = vec![0; len as usize];
        self.reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    fn value_bytes(&mut self, entry: Entry) -> Result<Vec<u8>, TiffError> {
        let len = entry.byte_len().ok_or(TiffError::Invalid)?;
        if len <= 4 {
            return Ok(entry.field[..len as usize].to_vec());
        }
        self.bytes(self.endian.u32(entry.field), len)
    }

    fn array<const N: usize>(&mut self, offset: u32) -> Result<[u8; N], TiffError> {
        let len = u32::try_from(N).map_err(|_| TiffError::Invalid)?;
        let bytes = self.bytes(offset, len)?;
        bytes.try_into().map_err(|_| TiffError::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    /// A little-endian TIFF with IFD0 at 8: Orientation 6, Make "Olympus",
    /// an Exif IFD with one SHORT entry, and IFD1 pointing back to IFD0.
    fn sample(endian: Endian) -> Vec<u8> {
        let mut out = Vec::new();
        let u16b = |v: u16| match endian {
            Endian::Little => v.to_le_bytes(),
            Endian::Big => v.to_be_bytes(),
        };
        let u32b = |v: u32| match endian {
            Endian::Little => v.to_le_bytes(),
            Endian::Big => v.to_be_bytes(),
        };
        out.extend_from_slice(if endian == Endian::Little {
            b"IIRO"
        } else {
            b"MMOR"
        });
        out.extend_from_slice(&u32b(8));
        // IFD0 at 8: 3 entries.
        let entry = |out: &mut Vec<u8>, tag: u16, kind: u16, count: u32, field: [u8; 4]| {
            out.extend_from_slice(&u16b(tag));
            out.extend_from_slice(&u16b(kind));
            out.extend_from_slice(&u32b(count));
            out.extend_from_slice(&field);
        };
        out.extend_from_slice(&u16b(3));
        let short = |v: u16| {
            let b = u16b(v);
            [b[0], b[1], 0, 0]
        };
        entry(&mut out, 0x0112, 3, 1, short(6));
        entry(&mut out, 0x010F, 2, 8, u32b(62));
        entry(&mut out, tag::EXIF_IFD, 4, 1, u32b(70));
        out.extend_from_slice(&u32b(90)); // next: IFD1
        assert_eq!(out.len(), 50);
        out.resize(62, 0);
        out.extend_from_slice(b"Olympus\0");
        // Exif IFD at 70: one entry.
        out.extend_from_slice(&u16b(1));
        entry(&mut out, 0xA002, 3, 1, short(4000));
        out.extend_from_slice(&u32b(0));
        out.resize(90, 0);
        // IFD1 at 90: one entry, next points back to IFD0 (a cycle).
        out.extend_from_slice(&u16b(1));
        entry(&mut out, 0x0103, 3, 1, short(6));
        out.extend_from_slice(&u32b(8));
        out
    }

    #[test]
    fn walks_the_chain_and_the_exif_ifd_in_both_byte_orders() {
        for endian in [Endian::Little, Endian::Big] {
            let (mut tiff, first) = TiffReader::open(Cursor::new(sample(endian))).unwrap();
            assert_eq!(tiff.endian(), endian);

            let ifds = tiff.walk(first);

            assert_eq!(ifds.len(), 3, "{endian:?}");
            let first_ifd = &ifds[0];
            assert_eq!(tiff.number(first_ifd.get(0x0112).unwrap()), Some(6));
            assert_eq!(
                tiff.text(first_ifd.get(0x010F).unwrap()).as_deref(),
                Some("Olympus")
            );
            assert!(ifds.iter().any(|ifd| ifd.get(0xA002).is_some()));
        }
    }

    #[test]
    fn refuses_data_that_is_not_tiff() {
        assert!(TiffReader::open(Cursor::new(b"\xFF\xD8\xFF\xE0 not tiff".to_vec())).is_err());
    }

    #[test]
    fn refuses_offsets_outside_the_file() {
        let (mut tiff, _) = TiffReader::open(Cursor::new(sample(Endian::Little))).unwrap();
        assert!(tiff.ifd(10_000).is_err());
        assert!(tiff.bytes(60, 100).is_err());
        assert_eq!(tiff.walk(10_000), Vec::new());
    }
}
