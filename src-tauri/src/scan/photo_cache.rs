//! The cache of photo metadata shared by scans, and cleaning the thumbnail
//! cache with it.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use sito_core::photo_cache::{CacheError, PhotoCache, SharedCache};
use sito_core::scan::ScanError;
use sito_core::timeline::{self, CachedScan, Photo, ReadMode, Timeline, TimelineProgress};

use crate::thumbnails::Thumbnails;

/// The cached photos of a folder whose EXIF is still pending.
#[derive(Debug, Default)]
pub struct Pending {
    /// The pending photos, in path order.
    pub photos: Vec<Photo>,
    /// How many photos of the folder have their EXIF read already.
    pub read: usize,
}

/// The photo cache, or `None` if it could not be opened; scans then read
/// every photo, as before the cache existed.
#[derive(Debug)]
pub struct PhotoCacheState(SharedCache);

impl PhotoCacheState {
    /// Opens the cache in `file` (see [`open_cache`]).
    pub fn open(file: &Path) -> Self {
        Self(Mutex::new(open_cache(file)))
    }

    /// Closes the cache file, so it can be moved; scans read every photo
    /// until [`Self::reopen`].
    pub fn close(&self) {
        *self.lock() = None;
    }

    /// Opens the cache in `file` in place of the one in use, as
    /// [`Self::open`] does.
    pub fn reopen(&self, file: &Path) {
        let cache = open_cache(file);
        *self.lock() = cache;
    }

    /// Removes every cached photo: every folder is read in full on its next
    /// scan. Without a cache there is nothing to remove.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache cannot be written.
    pub fn clear(&self) -> Result<(), CacheError> {
        self.lock().as_mut().map_or(Ok(()), PhotoCache::clear)
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
    /// Photos with a Takeout date file are dated from it alone and their
    /// EXIF left pending (see [`Self::pending`]). The cache stays usable by
    /// others meanwhile; `should_stop` can end the scan early. Also returns
    /// the photos found moved or renamed since the cache saw them.
    ///
    /// # Errors
    ///
    /// Returns an error if `root` cannot be scanned, and
    /// [`ScanError::Stopped`] once `should_stop` says so.
    pub fn scan(
        &self,
        root: &Path,
        should_stop: impl Fn() -> bool,
        on_progress: impl FnMut(TimelineProgress),
    ) -> Result<CachedScan, ScanError> {
        // Without a cache, pending photos could never be completed.
        let mode = if self.lock().is_some() {
            ReadMode::TakeoutFirst
        } else {
            ReadMode::Full
        };
        timeline::build_timeline_cached(root, &self.0, mode, should_stop, on_progress)
    }

    /// The cached photos of `root` whose EXIF is still pending, in path order.
    pub fn pending(&self, root: &Path) -> Pending {
        let cache = self.lock();
        let Some(cache) = cache.as_ref() else {
            return Pending::default();
        };
        let listed = cache
            .pending_under(root)
            .and_then(|photos| Ok((cache.photos_under(root)?.len(), photos)));
        match listed {
            Ok((all, photos)) => Pending {
                read: all.saturating_sub(photos.len()),
                photos,
            },
            Err(err) => {
                log::warn!("cannot list photos with pending EXIF: {err}");
                Pending::default()
            }
        }
    }

    /// Stores freshly read `photos` in the cache.
    pub fn store(&self, photos: &[Photo]) {
        let mut cache = self.lock();
        let Some(cache) = cache.as_mut() else {
            return;
        };
        if let Err(err) = cache.store(photos) {
            // They are read again next time.
            log::warn!("cannot cache {} photos: {err}", photos.len());
        }
    }

    /// Removes thumbnails of photos no longer in the cache and keeps the
    /// thumbnail cache under `limit` bytes.
    pub fn clean_thumbnails(&self, thumbnails: &Thumbnails, limit: u64) {
        // Without the list of known photos every thumbnail would look orphaned.
        let Some(Ok(needed)) = self.lock().as_ref().map(PhotoCache::all) else {
            return;
        };
        match thumbnails.cache().clean(&needed, limit) {
            Ok(report) => log::info!("thumbnail cache cleaned: {report:?}"),
            Err(err) => log::warn!("cannot clean the thumbnail cache: {err}"),
        }
    }
}

/// The cache in `file`, or `None` if it cannot be opened. A damaged file is
/// deleted and created anew: it only holds what can be read from the photos
/// again.
fn open_cache(file: &Path) -> Option<PhotoCache> {
    PhotoCache::open(file)
        .or_else(|err| {
            log::warn!("photo cache unusable, starting a new one: {err}");
            let _ = std::fs::remove_file(file);
            PhotoCache::open(file)
        })
        .map_err(|err| log::warn!("no photo cache: {err}"))
        .ok()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

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
        let file = sito_core::cache_dir::photo_cache_file(cache_dir.path());
        fs::write(&file, b"not a database").unwrap();

        let state = PhotoCacheState::open(&file);
        assert!(state.cached_timeline(dir.path()).is_none());
        let scanned = state.scan(dir.path(), || false, |_| {}).unwrap().timeline;

        let cached = state.cached_timeline(dir.path()).unwrap();
        assert_eq!(cached.photos, scanned.photos);
    }

    #[test]
    fn a_closed_cache_reopened_elsewhere_knows_what_was_moved_there() {
        let dir = gallery();
        let (old, new) = (TempDir::new().unwrap(), TempDir::new().unwrap());
        let file = sito_core::cache_dir::photo_cache_file(old.path());
        let state = PhotoCacheState::open(&file);
        state.scan(dir.path(), || false, |_| {}).unwrap();

        state.close();
        assert!(state.cached_timeline(dir.path()).is_none());
        let moved = sito_core::cache_dir::photo_cache_file(new.path());
        fs::rename(&file, &moved).unwrap();
        state.reopen(&moved);

        assert!(state.cached_timeline(dir.path()).is_some());
    }

    #[test]
    fn a_cleared_cache_knows_no_folder_and_keeps_working() {
        let dir = gallery();
        let cache_dir = TempDir::new().unwrap();
        let state =
            PhotoCacheState::open(&sito_core::cache_dir::photo_cache_file(cache_dir.path()));
        state.scan(dir.path(), || false, |_| {}).unwrap();

        state.clear().unwrap();

        assert!(state.cached_timeline(dir.path()).is_none());
        state.scan(dir.path(), || false, |_| {}).unwrap();
        assert!(state.cached_timeline(dir.path()).is_some());
    }
}
