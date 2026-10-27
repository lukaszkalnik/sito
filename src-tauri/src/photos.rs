//! `photo://` protocol: serves the original files of the current scan's photos
//! for the full-size viewer.
//!
//! URL: `photo://localhost/<photo index>?scan=<scan id>` (macOS) or
//! `http://photo.localhost/<photo index>?scan=<scan id>` (Windows). The bytes
//! are served unchanged; the webview applies the EXIF orientation itself.

use std::path::Path;

use tauri::http::{Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use crate::state::AppState;
use crate::thumbnails::{error_response, jpeg_response, parse_request};

/// Name of the URI scheme.
pub const SCHEME: &str = "photo";

/// Handles one `photo://` request; the file is read on a blocking thread.
pub fn handle<R: Runtime>(
    ctx: &UriSchemeContext<'_, R>,
    request: &Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let photo = parse_request(request.uri())
        .and_then(|(scan_id, index)| ctx.app_handle().state::<AppState>().photo(scan_id, index));
    let Some(photo) = photo else {
        responder.respond(error_response(StatusCode::NOT_FOUND));
        return;
    };
    tauri::async_runtime::spawn_blocking(move || responder.respond(read_photo(&photo.path)));
}

/// The response for the photo file at `path`.
fn read_photo(path: &Path) -> Response<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => jpeg_response(bytes),
        Err(err) => {
            log::warn!("cannot read {}: {err}", path.display());
            error_response(StatusCode::NOT_FOUND)
        }
    }
}

#[cfg(test)]
mod tests {
    use tauri::http::header::CONTENT_TYPE;

    use super::*;

    #[test]
    fn serves_the_original_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("IMG_1.jpg");
        std::fs::write(&path, [0xFF, 0xD8, 0xFF, 0xD9]).unwrap();

        let response = read_photo(&path);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "image/jpeg");
        assert_eq!(response.body(), &[0xFF, 0xD8, 0xFF, 0xD9]);
    }

    #[test]
    fn missing_file_is_not_found() {
        let dir = tempfile::tempdir().unwrap();

        let response = read_photo(&dir.path().join("gone.jpg"));

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
