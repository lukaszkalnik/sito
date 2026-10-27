//! `thumb://` protocol: serves thumbnails of the current scan's photos.
//!
//! URL: `thumb://localhost/<photo index>?scan=<scan id>` (macOS) or
//! `http://thumb.localhost/<photo index>?scan=<scan id>` (Windows).

use std::path::PathBuf;

use sito_core::thumbnail::ThumbnailCache;
use tauri::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use tauri::http::{HeaderValue, Request, Response, StatusCode, Uri};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use crate::state::AppState;

/// Name of the URI scheme.
pub const SCHEME: &str = "thumb";

/// Threads rendering thumbnails. Few on purpose: decoding is memory hungry,
/// and the UI only asks for what is on screen.
const WORKERS: usize = 2;

/// Renders and caches thumbnails off the main thread.
#[derive(Debug)]
pub struct Thumbnails {
    cache: ThumbnailCache,
    pool: rayon::ThreadPool,
}

impl Thumbnails {
    /// Thumbnails cached in `cache_dir`.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker threads cannot be started.
    pub fn new(cache_dir: PathBuf) -> Result<Self, rayon::ThreadPoolBuildError> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(WORKERS)
            .thread_name(|i| format!("thumbnail-{i}"))
            .build()?;
        Ok(Self {
            cache: ThumbnailCache::new(cache_dir),
            pool,
        })
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
        .and_then(|(scan_id, index)| app.state::<AppState>().photo(scan_id, index));
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

/// Reads `(scan id, photo index)` from a thumbnail or photo URL.
pub(crate) fn parse_request(uri: &Uri) -> Option<(u64, usize)> {
    let index = uri.path().strip_prefix('/')?.parse().ok()?;
    let scan_id = uri
        .query()?
        .split('&')
        .find_map(|pair| pair.strip_prefix("scan="))?
        .parse()
        .ok()?;
    Some((scan_id, index))
}

pub(crate) fn jpeg_response(bytes: Vec<u8>) -> Response<Vec<u8>> {
    let mut response = Response::new(bytes);
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
    // A URL always names the same photo: the scan id changes with every scan.
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("max-age=31536000, immutable"),
    );
    response
}

pub(crate) fn error_response(status: StatusCode) -> Response<Vec<u8>> {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(uri: &str) -> Option<(u64, usize)> {
        parse_request(&uri.parse().unwrap())
    }

    #[test]
    fn parses_urls_of_both_platforms() {
        assert_eq!(
            parse("thumb://localhost/42?scan=1700000000000"),
            Some((1_700_000_000_000, 42))
        );
        assert_eq!(parse("http://thumb.localhost/0?scan=7"), Some((7, 0)));
        assert_eq!(parse("http://thumb.localhost/3?x=1&scan=7"), Some((7, 3)));
    }

    #[test]
    fn rejects_malformed_urls() {
        for uri in [
            "http://thumb.localhost/42",
            "http://thumb.localhost/?scan=7",
            "http://thumb.localhost/abc?scan=7",
            "http://thumb.localhost/1/2?scan=7",
            "http://thumb.localhost/-1?scan=7",
            "http://thumb.localhost/1?scan=x",
        ] {
            assert_eq!(parse(uri), None, "{uri}");
        }
    }

    #[test]
    fn jpeg_response_is_cacheable() {
        let response = jpeg_response(vec![1, 2, 3]);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "image/jpeg");
        assert!(
            response.headers()[CACHE_CONTROL]
                .to_str()
                .unwrap()
                .contains("immutable")
        );
        assert_eq!(response.body(), &[1, 2, 3]);
    }

    #[test]
    fn error_response_is_not_cached() {
        let response = error_response(StatusCode::NOT_FOUND);

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        assert_eq!(response.body(), &Vec::<u8>::new());
    }
}
