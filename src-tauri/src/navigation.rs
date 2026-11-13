//! Keeps every window on the app's own pages: the webview never navigates
//! to another address, so no page from elsewhere can run in a window that
//! may request `thumb://` and `photo://` images.

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime, Url};

/// The plugin refusing every navigation away from the app, in every window
/// (also those created at run time).
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("navigation")
        .on_navigation(|webview, url| {
            let dev_url = if tauri::is_dev() {
                webview.app_handle().config().build.dev_url.clone()
            } else {
                None
            };
            let allowed = is_app_page(url, dev_url.as_ref());
            if !allowed {
                log::warn!("refused to navigate to {url}");
            }
            allowed
        })
        .build()
}

/// Whether `url` is a page of the app itself: the bundled frontend
/// (`tauri://localhost`, on Windows `http://tauri.localhost`), the empty
/// page a webview starts from, or a page of the dev server at `dev_url`.
fn is_app_page(url: &Url, dev_url: Option<&Url>) -> bool {
    let bundled = match url.scheme() {
        "tauri" => url.host_str() == Some("localhost"),
        "http" | "https" => url.host_str() == Some("tauri.localhost"),
        _ => false,
    };
    bundled || url.as_str() == "about:blank" || dev_url.is_some_and(|dev| same_origin(url, dev))
}

/// Whether two addresses have the same scheme, host and port.
fn same_origin(a: &Url, b: &Url) -> bool {
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    fn allowed(text: &str) -> bool {
        is_app_page(&url(text), None)
    }

    #[test]
    fn the_bundled_pages_are_allowed() {
        assert!(allowed("tauri://localhost/"));
        assert!(allowed("tauri://localhost/index.html?window=viewer"));
        assert!(allowed("http://tauri.localhost/"));
        assert!(allowed("https://tauri.localhost/index.html"));
        assert!(allowed("about:blank"));
    }

    #[test]
    fn a_web_page_is_refused() {
        assert!(!allowed("https://github.com/lukaszkalnik/sito/releases"));
        assert!(!allowed("http://localhost:5173/"));
    }

    #[test]
    fn a_look_alike_host_is_refused() {
        assert!(!allowed("https://tauri.localhost.example.com/"));
        assert!(!allowed("https://example.com/tauri.localhost"));
        assert!(!allowed("tauri://example.com/"));
        assert!(!allowed("https://localhost/"));
    }

    #[test]
    fn other_schemes_are_refused() {
        assert!(!allowed("file:///etc/passwd"));
        assert!(!allowed("data:text/html,<p>hi</p>"));
        assert!(!allowed("javascript:alert(1)"));
        assert!(!allowed("photo://localhost/1/2"));
    }

    #[test]
    fn the_dev_server_is_allowed_only_when_given() {
        let dev = url("http://localhost:5173");

        assert!(is_app_page(
            &url("http://localhost:5173/?window=viewer"),
            Some(&dev)
        ));
        assert!(!is_app_page(&url("http://localhost:5174/"), Some(&dev)));
        assert!(!is_app_page(&url("https://localhost:5173/"), Some(&dev)));
        assert!(!is_app_page(&url("https://example.com/"), Some(&dev)));
    }
}
