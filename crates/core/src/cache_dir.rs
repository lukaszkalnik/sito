//! The folder of Sito's cache: what belongs to it, where it is, which
//! folders it may move to, how large it is, and moving it (see
//! [`move_cache`]).
//!
//! The cache holds only what can be rebuilt from the photos: the thumbnail
//! folder ([`THUMBNAILS_DIR`]) and the photo cache ([`PHOTO_CACHE_FILE`]).
//! Nothing else in the folder is Sito's cache (the default folder also
//! holds the webview's data on Windows), so nothing else is moved or
//! removed.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::thumbnail::thumbnail_files;

mod relocate;

pub use relocate::{MoveError, MoveOutcome, MoveProgress, move_cache};

/// The folder of the thumbnail cache inside the cache folder.
pub const THUMBNAILS_DIR: &str = "thumbnails";

/// The photo cache's file inside the cache folder.
pub const PHOTO_CACHE_FILE: &str = "photos.db";

/// The subfolder Sito creates in a folder the user chooses for the cache, so
/// emptying and moving the cache never touch anything else in it.
pub const CACHE_SUBFOLDER: &str = "Sito-cache";

/// A file in a cache folder of the user's choice that keeps media scanners
/// (and Sito's own scans of a folder above it) from showing thumbnails as
/// photos.
pub const NO_MEDIA_FILE: &str = ".nomedia";

/// The thumbnail folder of the cache in `dir`.
#[must_use]
pub fn thumbnails_dir(dir: &Path) -> PathBuf {
    dir.join(THUMBNAILS_DIR)
}

/// The photo cache's file of the cache in `dir`.
#[must_use]
pub fn photo_cache_file(dir: &Path) -> PathBuf {
    dir.join(PHOTO_CACHE_FILE)
}

/// The cache folder in the folder `chosen` by the user.
#[must_use]
pub fn target_in(chosen: &Path) -> PathBuf {
    chosen.join(CACHE_SUBFOLDER)
}

/// Where the cache is in this session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheLocation {
    /// The system's app cache folder.
    Default {
        /// The folder.
        dir: PathBuf,
    },
    /// A folder the user chose.
    Chosen {
        /// The folder (its [`CACHE_SUBFOLDER`]).
        dir: PathBuf,
    },
    /// The user chose a folder that is not there (its drive is unplugged,
    /// it was removed), so the default folder is used for this session.
    Fallback {
        /// The default folder, in use.
        dir: PathBuf,
        /// The chosen folder that is missing.
        chosen: PathBuf,
    },
}

impl CacheLocation {
    /// Where the cache is when the user's choice is `chosen` (`None`: the
    /// default), the default folder is `default`, and `is_dir` tells
    /// whether a folder is there. A chosen folder that is not there falls
    /// back to the default; nothing is created for it.
    #[must_use]
    pub fn resolve(
        chosen: Option<PathBuf>,
        default: PathBuf,
        is_dir: impl Fn(&Path) -> bool,
    ) -> Self {
        match chosen {
            None => Self::Default { dir: default },
            Some(chosen) if is_dir(&chosen) => Self::Chosen { dir: chosen },
            Some(chosen) => Self::Fallback {
                dir: default,
                chosen,
            },
        }
    }

    /// The folder in use.
    #[must_use]
    pub fn dir(&self) -> &Path {
        match self {
            Self::Default { dir } | Self::Chosen { dir } | Self::Fallback { dir, .. } => dir,
        }
    }
}

/// Why a folder cannot take the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RefusedFolder {
    /// Files cannot be created in it.
    #[error("the folder is not writable")]
    NotWritable,
    /// It is inside the folder open in Sito, whose scans would walk the cache.
    #[error("the folder is inside the open folder")]
    InsideOpenFolder,
    /// It is the cache folder in use, is inside it or holds it, or it is
    /// inside the default cache folder.
    #[error("the folder is the cache folder or inside it")]
    InsideCache,
    /// It is a symbolic link or a junction, which could lead anywhere.
    #[error("the folder is a link")]
    Link,
    /// Its path is not valid Unicode, so it cannot be kept as a setting.
    #[error("the folder's name cannot be stored")]
    UnsupportedName,
}

/// The folders a chosen cache folder is checked against.
#[derive(Debug, Clone, Copy)]
pub struct Surroundings<'a> {
    /// The cache folder in use.
    pub current: &'a Path,
    /// The system's app cache folder.
    pub default: &'a Path,
    /// The folder open in Sito, if any.
    pub open_folder: Option<&'a Path>,
}

/// The cache folder (in [`CACHE_SUBFOLDER`]) for the folder `chosen` by the
/// user, or why it cannot take the cache. Paths are compared after
/// resolving links and `..`, as far as they exist. Checks that files can be
/// created in `chosen` by creating and removing an empty probe file.
///
/// # Errors
///
/// Returns the [`RefusedFolder`] reason.
pub fn check_choice(chosen: &Path, around: Surroundings<'_>) -> Result<PathBuf, RefusedFolder> {
    let target = target_in(chosen);
    if target.to_str().is_none() {
        return Err(RefusedFolder::UnsupportedName);
    }
    if [chosen, target.as_path()].into_iter().any(is_link) {
        return Err(RefusedFolder::Link);
    }
    check_place(
        &resolved(&target),
        Surroundings {
            current: &resolved(around.current),
            default: &resolved(around.default),
            open_folder: around.open_folder.map(resolved).as_deref(),
        },
    )?;
    if !is_writable(chosen) {
        return Err(RefusedFolder::NotWritable);
    }
    Ok(target)
}

/// The rules on where the cache folder `target` may be, on resolved paths.
fn check_place(target: &Path, around: Surroundings<'_>) -> Result<(), RefusedFolder> {
    if around
        .open_folder
        .is_some_and(|open| target.starts_with(open))
    {
        return Err(RefusedFolder::InsideOpenFolder);
    }
    let nested = |a: &Path, b: &Path| a.starts_with(b) || b.starts_with(a);
    if nested(target, around.current) || target.starts_with(around.default) {
        return Err(RefusedFolder::InsideCache);
    }
    Ok(())
}

/// Whether `path` is a symbolic link or (on Windows) a junction.
fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
}

/// `path` with links and `..` resolved as far as it exists, the rest
/// appended unchanged.
fn resolved(path: &Path) -> PathBuf {
    let mut missing = Vec::new();
    let mut existing = path;
    loop {
        if let Ok(real) = fs::canonicalize(existing) {
            return missing
                .iter()
                .rev()
                .fold(real, |path, name| path.join(name));
        }
        let (Some(parent), Some(name)) = (existing.parent(), existing.file_name()) else {
            return path.to_path_buf();
        };
        missing.push(name);
        existing = parent;
    }
}

/// Whether a file can be created in the folder `dir`.
fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".sito-write-test-{}", std::process::id()));
    let created = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .is_ok();
    if created {
        // Only Sito's own, empty probe.
        let _ = fs::remove_file(&probe);
    }
    created
}

/// How much space the cache takes, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CacheSize {
    /// The cached thumbnails and display images.
    pub thumbnails: u64,
    /// The photo cache.
    pub photo_data: u64,
}

/// How much space the cache in `dir` takes. A missing folder or file counts
/// as empty.
///
/// # Errors
///
/// Returns an error if the thumbnail folder exists but cannot be listed.
pub fn cache_size(dir: &Path) -> io::Result<CacheSize> {
    let dir = &crate::long_path::verbatim(dir)?;
    let thumbnails = thumbnail_files(&thumbnails_dir(dir))?
        .iter()
        .map(|file| file.len)
        .sum();
    let photo_data = fs::metadata(photo_cache_file(dir)).map_or(0, |meta| meta.len());
    Ok(CacheSize {
        thumbnails,
        photo_data,
    })
}

/// Removes the cache in `dir`: the cached thumbnails (and the subfolders
/// they leave empty), the photo cache file, and, if `dir` is a
/// [`CACHE_SUBFOLDER`], its [`NO_MEDIA_FILE`] and the folder itself once
/// empty. Anything else stays. The photo cache must be closed.
///
/// # Errors
///
/// Returns an error if a cache file exists but cannot be removed; files
/// removed before it stay removed.
pub fn remove_cache(dir: &Path) -> io::Result<()> {
    let dir = &crate::long_path::verbatim(dir)?;
    let thumbnails = thumbnails_dir(dir);
    for file in thumbnail_files(&thumbnails)? {
        fs::remove_file(&file.path)?;
    }
    remove_empty_dirs(&thumbnails);
    remove_if_present(&photo_cache_file(dir))?;
    if dir.file_name().is_some_and(|name| name == CACHE_SUBFOLDER) {
        remove_if_present(&dir.join(NO_MEDIA_FILE))?;
        // Stays if anything else is in it.
        let _ = fs::remove_dir(dir);
    }
    Ok(())
}

/// Removes the file `path` if it is there.
fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}

/// Removes the empty subfolders of `dir`, one level deep, and `dir` itself
/// if that leaves it empty.
fn remove_empty_dirs(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            // Fails, as it should, for a folder that is not empty.
            let _ = fs::remove_dir(entry.path());
        }
    }
    let _ = fs::remove_dir(dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT: &str = "/cache/default";

    fn around<'a>(current: &'a Path, open: Option<&'a Path>) -> Surroundings<'a> {
        Surroundings {
            current,
            default: Path::new(DEFAULT),
            open_folder: open,
        }
    }

    #[test]
    fn the_target_is_a_subfolder_of_the_chosen_folder() {
        assert_eq!(
            target_in(Path::new("/d/Cache")),
            Path::new("/d/Cache/Sito-cache")
        );
    }

    #[test]
    fn a_folder_elsewhere_may_take_the_cache() {
        let current = Path::new(DEFAULT);
        let open = Path::new("/photos");
        assert_eq!(
            check_place(Path::new("/d/Sito-cache"), around(current, Some(open))),
            Ok(())
        );
        // A sibling of the open folder whose name starts the same.
        assert_eq!(
            check_place(
                Path::new("/photos-2/Sito-cache"),
                around(current, Some(open))
            ),
            Ok(())
        );
    }

    #[test]
    fn a_folder_inside_the_open_folder_is_refused() {
        let open = Path::new("/photos");
        for target in ["/photos/Sito-cache", "/photos/2024/Sito-cache"] {
            assert_eq!(
                check_place(Path::new(target), around(Path::new(DEFAULT), Some(open))),
                Err(RefusedFolder::InsideOpenFolder),
                "{target}"
            );
        }
    }

    #[test]
    fn the_cache_folder_and_folders_inside_or_around_it_are_refused() {
        let current = Path::new("/d/Sito-cache");
        for target in [
            "/d/Sito-cache",
            "/d/Sito-cache/x/Sito-cache",
            // Holds the cache in use.
            "/d",
        ] {
            assert_eq!(
                check_place(Path::new(target), around(current, None)),
                Err(RefusedFolder::InsideCache),
                "{target}"
            );
        }
    }

    #[test]
    fn a_folder_inside_the_default_folder_is_refused_also_when_not_in_use() {
        let current = Path::new("/d/Sito-cache");
        assert_eq!(
            check_place(
                Path::new("/cache/default/Sito-cache"),
                around(current, None)
            ),
            Err(RefusedFolder::InsideCache)
        );
    }

    #[test]
    fn a_chosen_folder_that_exists_is_used() {
        let location = CacheLocation::resolve(
            Some(PathBuf::from("/d/Sito-cache")),
            PathBuf::from(DEFAULT),
            |_| true,
        );
        assert_eq!(
            location,
            CacheLocation::Chosen {
                dir: PathBuf::from("/d/Sito-cache")
            }
        );
        assert_eq!(location.dir(), Path::new("/d/Sito-cache"));
    }

    #[test]
    fn a_missing_chosen_folder_falls_back_to_the_default() {
        let location = CacheLocation::resolve(
            Some(PathBuf::from("/d/Sito-cache")),
            PathBuf::from(DEFAULT),
            |_| false,
        );
        assert_eq!(
            location,
            CacheLocation::Fallback {
                dir: PathBuf::from(DEFAULT),
                chosen: PathBuf::from("/d/Sito-cache"),
            }
        );
        assert_eq!(location.dir(), Path::new(DEFAULT));
    }

    #[test]
    fn without_a_choice_the_default_is_used() {
        let location = CacheLocation::resolve(None, PathBuf::from(DEFAULT), |_| false);
        assert_eq!(
            location,
            CacheLocation::Default {
                dir: PathBuf::from(DEFAULT)
            }
        );
    }

    #[test]
    fn a_writable_folder_elsewhere_gets_its_subfolder() {
        let root = tempfile::tempdir().unwrap();
        let chosen = root.path().join("Pamięć – ąę");
        fs::create_dir(&chosen).unwrap();
        let current = root.path().join("current");

        let target = check_choice(&chosen, around(&current, None)).unwrap();

        assert_eq!(target, chosen.join(CACHE_SUBFOLDER));
        // The probe is gone again and nothing is created yet.
        assert_eq!(fs::read_dir(&chosen).unwrap().count(), 0);
    }

    #[test]
    fn the_cache_folder_in_use_is_refused_also_through_dot_dot() {
        let root = tempfile::tempdir().unwrap();
        let current = root.path().join("d").join(CACHE_SUBFOLDER);
        fs::create_dir_all(&current).unwrap();
        let chosen = current.join("..");

        assert_eq!(
            check_choice(&chosen, around(&current, None)),
            Err(RefusedFolder::InsideCache)
        );
    }

    #[test]
    fn a_folder_that_cannot_be_written_is_refused() {
        let root = tempfile::tempdir().unwrap();
        // A folder that does not exist cannot take a file.
        let chosen = root.path().join("missing");

        assert_eq!(
            check_choice(&chosen, around(&root.path().join("current"), None)),
            Err(RefusedFolder::NotWritable)
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_link_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let real = root.path().join("real");
        fs::create_dir(&real).unwrap();
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let current = root.path().join("current");

        assert_eq!(
            check_choice(&link, around(&current, None)),
            Err(RefusedFolder::Link)
        );
        // A `Sito-cache` that is a link, in a real folder.
        std::os::unix::fs::symlink(&real, real.join(CACHE_SUBFOLDER)).unwrap();
        assert_eq!(
            check_choice(&real, around(&current, None)),
            Err(RefusedFolder::Link)
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_name_that_is_not_unicode_is_refused() {
        use std::os::unix::ffi::OsStrExt;

        let chosen = Path::new(std::ffi::OsStr::from_bytes(b"/tmp/\xff"));
        assert_eq!(
            check_choice(chosen, around(Path::new("/c"), None)),
            Err(RefusedFolder::UnsupportedName)
        );
    }

    #[test]
    fn the_size_sums_thumbnails_and_photo_data() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path();
        assert_eq!(cache_size(dir).unwrap(), CacheSize::default());
        let hex = "ab".repeat(32);
        let sub = thumbnails_dir(dir).join("ab");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join(format!("{hex}.jpg")), [0; 300]).unwrap();
        fs::write(sub.join(format!("{}.jpg", "cd".repeat(32))), [0; 200]).unwrap();
        // Not a cache file.
        fs::write(sub.join("notes.txt"), [0; 1000]).unwrap();
        fs::write(photo_cache_file(dir), [0; 40]).unwrap();

        assert_eq!(
            cache_size(dir).unwrap(),
            CacheSize {
                thumbnails: 500,
                photo_data: 40
            }
        );
    }
}
