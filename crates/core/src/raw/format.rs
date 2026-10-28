//! The RAW formats Sito reads.

use std::path::Path;

/// A camera RAW format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RawFormat {
    /// Olympus / OM System `.orf`.
    Orf,
    /// Canon `.cr2`.
    Cr2,
    /// Nikon `.nef`.
    Nef,
    /// Sony `.arw`.
    Arw,
    /// Panasonic `.rw2`.
    Rw2,
    /// Fujifilm `.raf`.
    Raf,
    /// Adobe Digital Negative `.dng`.
    Dng,
}

impl RawFormat {
    /// Every supported format.
    pub const ALL: [Self; 7] = [
        Self::Orf,
        Self::Cr2,
        Self::Nef,
        Self::Arw,
        Self::Rw2,
        Self::Raf,
        Self::Dng,
    ];

    /// The file extension, lower case, without the dot.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Orf => "orf",
            Self::Cr2 => "cr2",
            Self::Nef => "nef",
            Self::Arw => "arw",
            Self::Rw2 => "rw2",
            Self::Raf => "raf",
            Self::Dng => "dng",
        }
    }

    /// The format of a file with this extension (any letter case).
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        Self::ALL
            .into_iter()
            .find(|format| ext.eq_ignore_ascii_case(format.extension()))
    }

    /// Whether the file is a TIFF structure (every format but RAF).
    #[must_use]
    pub fn is_tiff_based(self) -> bool {
        match self {
            Self::Orf | Self::Cr2 | Self::Nef | Self::Arw | Self::Rw2 | Self::Dng => true,
            Self::Raf => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_extensions_in_any_case() {
        assert_eq!(
            RawFormat::from_path(Path::new("P1010001.ORF")),
            Some(RawFormat::Orf)
        );
        assert_eq!(
            RawFormat::from_path(Path::new("a.dng")),
            Some(RawFormat::Dng)
        );
        assert_eq!(RawFormat::from_path(Path::new("a.cr3")), None);
        assert_eq!(RawFormat::from_path(Path::new("a.jpg")), None);
        assert_eq!(RawFormat::from_path(Path::new("orf")), None);
    }
}
