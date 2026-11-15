//! Recognising photos that were moved or renamed outside Sito: a photo gone
//! from its old path whose file shows up at a new path with the same name,
//! size and modification time.

use std::collections::HashMap;
use std::ffi::OsString;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What identifies a photo file apart from its folder: its file name, size
/// and modification time. Two files with the same identity are taken to be
/// the same file, one moved from the other's place.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    name: OsString,
    size: u64,
    modified: SystemTime,
}

impl FileIdentity {
    /// The identity of the file at `path` with `size` and `modified`;
    /// `None` without a file name or a modification time, since such a file
    /// cannot be told apart from others reliably.
    #[must_use]
    pub fn new(path: &Path, size: u64, modified: Option<SystemTime>) -> Option<Self> {
        Some(Self {
            name: path.file_name()?.to_os_string(),
            size,
            modified: modified?,
        })
    }

    /// The file size in bytes.
    #[must_use]
    pub fn size(&self) -> u64 {
        self.size
    }

    /// The modification time.
    #[must_use]
    pub fn modified(&self) -> SystemTime {
        self.modified
    }

    /// Whether `path` has this identity's file name.
    #[must_use]
    pub fn has_name_of(&self, path: &Path) -> bool {
        path.file_name() == Some(self.name.as_os_str())
    }
}

/// A photo moved or renamed outside Sito.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedPhoto {
    /// Where the photo was before.
    pub from: PathBuf,
    /// Where it is now.
    pub to: PathBuf,
}

/// Pairs the `gone` files with the `appeared` ones of the same identity,
/// as `(gone index, appeared index)`, in the order of `gone`. Only unique
/// matches count: an identity that two gone or two appeared files share is
/// left out, and so is every file without an identity.
#[must_use]
pub fn match_moved(
    gone: &[Option<FileIdentity>],
    appeared: &[Option<FileIdentity>],
) -> Vec<(usize, usize)> {
    let gone_by_identity = unique_positions(gone);
    let appeared_by_identity = unique_positions(appeared);
    let mut pairs: Vec<(usize, usize)> = gone_by_identity
        .into_iter()
        .filter_map(|(identity, gone)| Some((gone?, (*appeared_by_identity.get(identity)?)?)))
        .collect();
    pairs.sort_unstable();
    pairs
}

/// For each value, its position if it occurs once, `None` if more often.
/// Missing values are left out.
pub(crate) fn unique_positions<T: Eq + Hash>(values: &[Option<T>]) -> HashMap<&T, Option<usize>> {
    let mut positions: HashMap<&T, Option<usize>> = HashMap::new();
    for (index, value) in values.iter().enumerate() {
        let Some(value) = value else { continue };
        positions
            .entry(value)
            .and_modify(|position| *position = None)
            .or_insert(Some(index));
    }
    positions
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::*;

    fn time(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }

    fn identity(path: &str, size: u64, seconds: u64) -> Option<FileIdentity> {
        FileIdentity::new(Path::new(path), size, Some(time(seconds)))
    }

    #[test]
    fn matches_a_renamed_folder() {
        let gone = [
            identity("/a/2023/IMG_1.jpg", 10, 1),
            identity("/a/2023/IMG_2.jpg", 20, 2),
        ];
        let appeared = [
            identity("/a/Rok 2023/IMG_2.jpg", 20, 2),
            identity("/a/Rok 2023/IMG_1.jpg", 10, 1),
        ];

        assert_eq!(match_moved(&gone, &appeared), vec![(0, 1), (1, 0)]);
    }

    #[test]
    fn matches_a_photo_moved_to_another_subfolder() {
        let gone = [identity("/a/x/IMG_1.jpg", 10, 1)];
        let appeared = [
            identity("/a/new.jpg", 5, 3),
            identity("/a/y/z/IMG_1.jpg", 10, 1),
        ];

        assert_eq!(match_moved(&gone, &appeared), vec![(0, 1)]);
    }

    #[test]
    fn leaves_out_identities_shared_by_two_files() {
        let gone = [
            identity("/a/x/IMG_1.jpg", 10, 1),
            identity("/a/y/IMG_1.jpg", 10, 1),
        ];
        let appeared = [identity("/b/IMG_1.jpg", 10, 1)];
        assert_eq!(match_moved(&gone, &appeared), vec![]);

        let gone = [identity("/a/x/IMG_1.jpg", 10, 1)];
        let appeared = [
            identity("/b/IMG_1.jpg", 10, 1),
            identity("/c/IMG_1.jpg", 10, 1),
        ];
        assert_eq!(match_moved(&gone, &appeared), vec![]);
    }

    #[test]
    fn never_matches_a_file_without_a_modification_time() {
        let gone = [FileIdentity::new(Path::new("/a/IMG_1.jpg"), 10, None)];
        let appeared = [FileIdentity::new(Path::new("/b/IMG_1.jpg"), 10, None)];

        assert_eq!(gone, [None]);
        assert_eq!(match_moved(&gone, &appeared), vec![]);
    }

    #[test]
    fn does_not_match_another_size_time_or_name() {
        let gone = [identity("/a/IMG_1.jpg", 10, 1)];
        for appeared in [
            identity("/b/IMG_1.jpg", 11, 1),
            identity("/b/IMG_1.jpg", 10, 2),
            identity("/b/IMG_2.jpg", 10, 1),
        ] {
            assert_eq!(match_moved(&gone, &[appeared]), vec![]);
        }
    }

    #[test]
    fn tells_whether_a_path_has_the_same_name() {
        let identity = identity("/a/IMG_1.jpg", 10, 1).unwrap();

        assert!(identity.has_name_of(Path::new("/z/IMG_1.jpg")));
        assert!(!identity.has_name_of(Path::new("/a/IMG_2.jpg")));
    }
}
