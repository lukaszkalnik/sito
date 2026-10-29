//! `photo://` protocol: serves the current scan's photos for the full-size
//! viewer as JPEGs: a JPEG's own bytes, a RAW file's embedded preview.
//!
//! URL: `photo://localhost/<photo id>?scan=<scan id>` (macOS) or
//! `http://photo.localhost/<photo id>?scan=<scan id>` (Windows). The webview
//! applies the EXIF orientation itself.

use sito_core::display::{DisplayError, display_jpeg};
use sito_core::thumbnail::ThumbnailCache;
use sito_core::timeline::Photo;

use tauri::http::{Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use crate::protocol::{error_response, jpeg_response, parse_request};
use crate::scan::state::AppState;
use crate::thumbnails::Thumbnails;

/// Name of the URI scheme.
pub const SCHEME: &str = "photo";

/// Handles one `photo://` request; the file is read on a blocking thread.
pub fn handle<R: Runtime>(
    ctx: &UriSchemeContext<'_, R>,
    request: &Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let photo = parse_request(request.uri())
        .and_then(|(scan_id, id)| ctx.app_handle().state::<AppState>().photo(scan_id, id));
    let Some(photo) = photo else {
        responder.respond(error_response(StatusCode::NOT_FOUND));
        return;
    };
    let cache = ctx.app_handle().state::<Thumbnails>().cache().clone();
    tauri::async_runtime::spawn_blocking(move || responder.respond(read_photo(&photo, &cache)));
}

/// The response for `photo`.
fn read_photo(photo: &Photo, cache: &ThumbnailCache) -> Response<Vec<u8>> {
    match display_jpeg(photo, cache) {
        Ok(bytes) => jpeg_response(bytes),
        Err(err) => {
            log::warn!("cannot show {}: {err}", photo.path.display());
            let status = match err {
                DisplayError::Io { .. } => StatusCode::NOT_FOUND,
                DisplayError::Raw { .. }
                | DisplayError::Convert { .. }
                | DisplayError::NotAPhoto(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            };
            error_response(status)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sito_core::metadata::Orientation;
    use sito_core::timeline::{ExifInfo, TimeSource};
    use tauri::http::header::CONTENT_TYPE;

    use super::*;

    fn photo(path: PathBuf) -> Photo {
        Photo {
            path,
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
    fn serves_the_original_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("IMG_1.jpg");
        std::fs::write(&path, [0xFF, 0xD8, 0xFF, 0xD9]).unwrap();

        let response = read_photo(&photo(path), &ThumbnailCache::new(dir.path().join("cache")));

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "image/jpeg");
        assert_eq!(response.body(), &[0xFF, 0xD8, 0xFF, 0xD9]);
    }

    #[test]
    fn missing_file_is_not_found() {
        let dir = tempfile::tempdir().unwrap();

        let response = read_photo(
            &photo(dir.path().join("gone.jpg")),
            &ThumbnailCache::new(dir.path().join("cache")),
        );

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
