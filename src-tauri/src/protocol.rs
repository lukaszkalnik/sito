//! Shared parts of the `thumb://` and `photo://` protocols: parsing request
//! URLs and building responses.

use tauri::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use tauri::http::{HeaderValue, Response, StatusCode, Uri};

/// Reads `(scan id, photo id)` from a thumbnail or photo URL.
pub fn parse_request(uri: &Uri) -> Option<(u64, usize)> {
    let id = uri.path().strip_prefix('/')?.parse().ok()?;
    let scan_id = uri
        .query()?
        .split('&')
        .find_map(|pair| pair.strip_prefix("scan="))?
        .parse()
        .ok()?;
    Some((scan_id, id))
}

/// A JPEG response that the webview may cache forever.
pub fn jpeg_response(bytes: Vec<u8>) -> Response<Vec<u8>> {
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

/// An empty response with `status` that the webview must not cache.
pub fn error_response(status: StatusCode) -> Response<Vec<u8>> {
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
