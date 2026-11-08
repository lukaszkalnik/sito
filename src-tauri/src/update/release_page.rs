//! Addresses of Sito's pages on GitHub that the app may open in the system
//! browser.

use tauri::Url;

/// Sito's repository on GitHub, the only place the app opens pages of.
const REPOSITORY: &str = "https://github.com/lukaszkalnik/sito/";
/// The owner and name of [`REPOSITORY`], as the first two path segments.
const REPOSITORY_PATH: [&str; 2] = ["lukaszkalnik", "sito"];

/// The GitHub release page of `version` (without the leading `v`), e.g.
/// `https://github.com/lukaszkalnik/sito/releases/tag/v0.15.1`. The version
/// is one path segment, so `/`, `?` or `#` in it are escaped.
///
/// `None` only if the address built is not a [`github_page`], which would be
/// a bug.
pub fn release_page(version: &str) -> Option<Url> {
    let mut url = Url::parse(REPOSITORY).ok()?;
    url.path_segments_mut().ok()?.pop_if_empty().extend([
        "releases",
        "tag",
        &format!("v{version}"),
    ]);
    github_page(&url).then_some(url)
}

/// Whether `url` is a page of Sito's repository on GitHub: `https`, host
/// `github.com` without a port or user, path in `/lukaszkalnik/sito`.
pub fn github_page(url: &Url) -> bool {
    let in_repository = url.path_segments().is_some_and(|segments| {
        segments
            .take(REPOSITORY_PATH.len())
            .eq(REPOSITORY_PATH.iter().copied())
    });
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && in_repository
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(version: &str) -> Option<String> {
        release_page(version).map(String::from)
    }

    fn is_github_page(url: &str) -> bool {
        Url::parse(url).is_ok_and(|url| github_page(&url))
    }

    #[test]
    fn the_release_page_is_the_tag_of_the_version() {
        assert_eq!(
            page("0.15.1").as_deref(),
            Some("https://github.com/lukaszkalnik/sito/releases/tag/v0.15.1")
        );
    }

    #[test]
    fn a_version_cannot_leave_the_path() {
        assert_eq!(
            page("1.0.0/../../../evil?x=1#y").as_deref(),
            Some(
                "https://github.com/lukaszkalnik/sito/releases/tag/v1.0.0%2F..%2F..%2F..%2Fevil%3Fx=1%23y"
            )
        );
    }

    #[test]
    fn pages_of_the_repository_are_accepted() {
        assert!(is_github_page(
            "https://github.com/lukaszkalnik/sito/releases/tag/v0.15.1"
        ));
        assert!(is_github_page(
            "https://github.com/lukaszkalnik/sito/blob/main/docs/slices/41-update-check.md"
        ));
    }

    #[test]
    fn other_addresses_are_refused() {
        assert!(!is_github_page(
            "http://github.com/lukaszkalnik/sito/releases"
        ));
        assert!(!is_github_page(
            "https://github.com.evil.com/lukaszkalnik/sito/releases"
        ));
        assert!(!is_github_page(
            "https://evil@github.com/lukaszkalnik/sito/releases"
        ));
        assert!(!is_github_page(
            "https://github.com:8443/lukaszkalnik/sito/releases"
        ));
        assert!(!is_github_page("https://github.com/someone/sito/releases"));
        assert!(!is_github_page(
            "https://github.com/lukaszkalnik/sito-fork/releases"
        ));
        assert!(!is_github_page("file:///lukaszkalnik/sito/releases"));
    }
}
