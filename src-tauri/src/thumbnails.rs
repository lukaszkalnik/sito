//! `thumb://` protocol: serves thumbnails of the current scan's photos.
//!
//! URL: `thumb://localhost/<photo id>?scan=<scan id>` (macOS) or
//! `http://thumb.localhost/<photo id>?scan=<scan id>` (Windows).

use std::path::PathBuf;

use sito_core::thumbnail::ThumbnailCache;
use tauri::http::{Request, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use crate::protocol::{error_response, jpeg_response, parse_request};
use crate::scan::state::AppState;
use lifo_pool::LifoPool;

mod lifo_pool;

/// Name of the URI scheme.
pub const SCHEME: &str = "thumb";

/// Threads rendering thumbnails. Few on purpose: decoding is memory hungry,
/// and the UI only asks for what is on screen. The newest request is served
/// first, so after a fast scroll the photos on screen come before those
/// scrolled past.
const WORKERS: usize = 2;

/// Renders and caches thumbnails off the main thread.
#[derive(Debug)]
pub struct Thumbnails {
    cache: ThumbnailCache,
    pool: LifoPool,
}

impl Thumbnails {
    /// Thumbnails cached in `cache_dir`.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker threads cannot be started.
    pub fn new(cache_dir: PathBuf) -> std::io::Result<Self> {
        let pool = LifoPool::new(WORKERS, "thumbnail")?;
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
    let photo = parse_request(request.uri())
        .and_then(|(scan_id, id)| app.state::<AppState>().photo(scan_id, id));
    let Some(photo) = photo else {
        responder.respond(error_response(StatusCode::NOT_FOUND));
        return;
    };

    let cache = thumbnails.cache.clone();
    thumbnails.pool.spawn(move || {
        let response = match cache.get_or_create(&photo) {
            Ok(bytes) => jpeg_response(bytes),
            Err(err) => {
                log::warn!("thumbnail failed: {err:?}");
                error_response(StatusCode::INTERNAL_SERVER_ERROR)
            }
        };
        responder.respond(response);
    });
}
