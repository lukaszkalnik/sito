//! `thumb://` protocol: serves thumbnails of the current scan's photos.
//!
//! URL: `thumb://localhost/<photo id>?scan=<scan id>` (macOS) or
//! `http://thumb.localhost/<photo id>?scan=<scan id>` (Windows).

use std::path::PathBuf;

use sito_core::thumbnail::ThumbnailCache;
use sito_core::timeline::Photo;
use tauri::http::{Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use crate::protocol::{error_response, jpeg_response, parse_request};
use crate::scan::state::AppState;
use worker_pool::WorkerPool;

mod worker_pool;

/// Name of the URI scheme.
pub const SCHEME: &str = "thumb";

/// Threads rendering thumbnails. Few on purpose: decoding is memory hungry,
/// and the UI asks for only a few thumbnails at a time, visible ones first.
const WORKERS: usize = 2;

/// Renders and caches thumbnails off the main thread.
#[derive(Debug)]
pub struct Thumbnails {
    cache: ThumbnailCache,
    pool: WorkerPool,
}

impl Thumbnails {
    /// Thumbnails cached in `cache_dir`.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker threads cannot be started.
    pub fn new(cache_dir: PathBuf) -> std::io::Result<Self> {
        let pool = WorkerPool::new(WORKERS, "thumbnail")?;
        Ok(Self {
            cache: ThumbnailCache::new(cache_dir),
            pool,
        })
    }

    /// The thumbnail cache on disk.
    pub fn cache(&self) -> &ThumbnailCache {
        &self.cache
    }
}

/// Handles one `thumb://` request; the response is sent from a worker thread.
pub fn handle<R: Runtime>(
    ctx: &UriSchemeContext<'_, R>,
    request: &Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx.app_handle();
    let Some(thumbnails) = app.try_state::<Thumbnails>() else {
        responder.respond(error_response(StatusCode::SERVICE_UNAVAILABLE));
        return;
    };
    let Some((scan_id, id)) = parse_request(request.uri()) else {
        responder.respond(error_response(StatusCode::NOT_FOUND));
        return;
    };
    let Some(photo) = app.state::<AppState>().photo(scan_id, id) else {
        responder.respond(error_response(StatusCode::NOT_FOUND));
        return;
    };

    let cache = thumbnails.cache.clone();
    let app = app.clone();
    thumbnails.pool.spawn(move || {
        let still_shown = || app.state::<AppState>().is_current(scan_id);
        responder.respond(thumbnail_response(&cache, &photo, still_shown));
    });
}

/// The thumbnail of `photo`, or `410 Gone` without rendering it if its scan
/// is `still_shown` no more: after opening another folder, the requests
/// still queued would otherwise keep the workers busy for nothing (minutes
/// on a slow disk) while the new folder's thumbnails wait.
fn thumbnail_response(
    cache: &ThumbnailCache,
    photo: &Photo,
    still_shown: impl Fn() -> bool,
) -> Response<Vec<u8>> {
    if !still_shown() {
        return error_response(StatusCode::GONE);
    }
    match cache.get_or_create(photo) {
        Ok(bytes) => jpeg_response(bytes),
        Err(err) => {
            log::warn!("thumbnail failed: {err:?}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sito_core::metadata::Orientation;
    use sito_core::timeline::{ExifInfo, TimeSource};

    use super::*;

    fn missing_photo() -> Photo {
        Photo {
            path: PathBuf::from("missing.jpg"),
            size: 4,
            modified: None,
            taken_at: chrono::NaiveDateTime::default(),
            time_source: TimeSource::Exif,
            exif: ExifInfo::Read {
                orientation: Orientation::NORMAL,
                camera: None,
            },
            sidecar: None,
            companion: None,
        }
    }

    #[test]
    fn a_thumbnail_of_a_scan_no_longer_shown_is_not_rendered() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ThumbnailCache::new(dir.path().to_path_buf());

        let gone = thumbnail_response(&cache, &missing_photo(), || false);
        let tried = thumbnail_response(&cache, &missing_photo(), || true);

        assert_eq!(gone.status(), StatusCode::GONE);
        assert_eq!(tried.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
