//! The cache of photo metadata shared by scans, and cleaning the thumbnail
//! cache with it.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use sito_core::photo_cache::PhotoCache;
use sito_core::scan::ScanError;
use sito_core::timeline::{self, Timeline, TimelineProgress};

use crate::thumbnails::Thumbnails;

/// Largest size of the thumbnail cache, in bytes; older thumbnails are
/// removed beyond it. Slice 13 turns it into a setting.
pub const THUMBNAIL_CACHE_LIMIT: u64 = 500 * 1024 * 1024;

/// The photo cache, or `None` if it could not be opened; scans then read
/// every photo, as before the cache existed.
#[derive(Debug)]
pub struct PhotoCacheState(Mutex<Option<PhotoCache>>);

impl PhotoCacheState {
    /// Opens the cache in `file`. A damaged file is deleted and created anew:
    /// it only holds what can be read from the photos again.
    pub fn open(file: &Path) -> Self {
        let cache = PhotoCache::open(file).or_else(|err| {
            log::warn!("photo cache unusable, starting a new one: {err}");
            let _ = std::fs::remove_file(file);
            PhotoCache::open(file)
        });
        Self(Mutex::new(
            cache
                .map_err(|err| log::warn!("no photo cache: {err}"))
                .ok(),
        ))
    }

    fn lock(&self) -> MutexGuard<'_, Option<PhotoCache>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The timeline of `root` as last scanned, if the cache knows any photo
    /// in it.
    pub fn cached_timeline(&self, root: &Path) -> Option<Timeline> {
        let cache = self.lock();
        let timeline = timeline::cached_timeline(root, cache.as_ref()?).ok()?;
        (!timeline.photos.is_empty() || !timeline.rejected.is_empty()).then_some(timeline)
    }

    /// Scans `root`, reading only photos the cache does not know unchanged.
    ///
    /// # Errors
    ///
    /// Returns an error if `root` cannot be scanned.
    pub fn scan(
        &self,
        root: &Path,
        on_progress: impl FnMut(TimelineProgress),
    ) -> Result<Timeline, ScanError> {
        let mut cache = self.lock();
        timeline::build_timeline_cached(root, cache.as_mut(), on_progress)
    }

    /// Removes thumbnails of photos no longer in the cache and keeps the
    /// thumbnail cache under [`THUMBNAIL_CACHE_LIMIT`].
    pub fn clean_thumbnails(&self, thumbnails: &Thumbnails) {
        // Without the list of known photos every thumbnail would look orphaned.
        let Some(Ok(needed)) = self.lock().as_ref().map(PhotoCache::all) else {
            return;
        };
        match thumbnails.cache().clean(&needed, THUMBNAIL_CACHE_LIMIT) {
            Ok(report) => log::info!("thumbnail cache cleaned: {report:?}"),
            Err(err) => log::warn!("cannot clean the thumbnail cache: {err}"),
        }
    }
}

/// Where the photo cache lives: next to the thumbnail cache.
pub fn cache_file(cache_dir: &Path) -> PathBuf {
    cache_dir.join("photos.db")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    /// A temp folder with copies of the EXIF fixtures.
    fn gallery() -> TempDir {
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/exif");
        let dir = TempDir::new().unwrap();
        for entry in fs::read_dir(fixtures).unwrap() {
            let entry = entry.unwrap();
            fs::copy(entry.path(), dir.path().join(entry.file_name())).unwrap();
        }
        dir
    }

    #[test]
    fn a_damaged_cache_file_is_replaced_by_a_working_one() {
        let dir = gallery();
        let cache_dir = TempDir::new().unwrap();
        let file = cache_file(cache_dir.path());
        fs::write(&file, b"not a database").unwrap();

        let state = PhotoCacheState::open(&file);
        assert!(state.cached_timeline(dir.path()).is_none());
        let scanned = state.scan(dir.path(), |_| {}).unwrap();

        let cached = state.cached_timeline(dir.path()).unwrap();
        assert_eq!(cached.photos, scanned.photos);
    }
}
