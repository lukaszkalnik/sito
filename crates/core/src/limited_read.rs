//! Reading a whole file into memory, up to a size limit: a file from the
//! gallery can be any size, whatever its name says.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Largest JPEG read whole for viewing. Real photos are far smaller; a file
/// beyond this is not shown rather than risking the machine's memory.
pub(crate) const MAX_JPEG_BYTES: u64 = 512 * 1024 * 1024;

/// Why a file was not read.
#[derive(Debug, thiserror::Error)]
pub(crate) enum LimitedReadError {
    /// The file could not be read.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// The file is larger than the limit.
    #[error("the file is larger than {max} bytes")]
    TooLarge {
        /// The limit, in bytes.
        max: u64,
    },
}

/// The contents of the file at `path`, if it is at most `max` bytes long.
///
/// # Errors
///
/// [`LimitedReadError::TooLarge`] for a longer file, of which nothing is
/// read; [`LimitedReadError::Io`] if the file cannot be read.
pub(crate) fn read_limited(path: &Path, max: u64) -> Result<Vec<u8>, LimitedReadError> {
    let file = File::open(path)?;
    if file.metadata()?.len() > max {
        return Err(LimitedReadError::TooLarge { max });
    }
    // The file may grow after the check, so the read itself stops just past
    // the limit as well.
    let mut bytes = Vec::new();
    file.take(max.saturating_add(1)).read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).is_ok_and(|len| len > max) {
        return Err(LimitedReadError::TooLarge { max });
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn reads_a_file_up_to_the_limit() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("zdjęcie.jpg");
        std::fs::write(&path, b"12345").unwrap();

        assert_eq!(read_limited(&path, 5).unwrap(), b"12345");
    }

    #[test]
    fn refuses_a_file_over_the_limit() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("big.jpg");
        std::fs::write(&path, b"123456").unwrap();

        assert!(matches!(
            read_limited(&path, 5),
            Err(LimitedReadError::TooLarge { max: 5 })
        ));
    }

    #[test]
    fn reports_a_missing_file() {
        let dir = TempDir::new().unwrap();

        assert!(matches!(
            read_limited(&dir.path().join("gone.jpg"), 5),
            Err(LimitedReadError::Io(_))
        ));
    }
}
