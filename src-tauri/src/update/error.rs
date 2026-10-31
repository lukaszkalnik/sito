//! The error codes of the update commands.

use serde::Serialize;

/// Error sent to the UI when checking for or installing an update fails.
/// Only a code is sent; the UI shows a translated message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "camelCase")]
pub enum UpdateError {
    /// GitHub could not be reached, or its answer was not a usable manifest.
    Network,
    /// The download's signature does not match Sito's public key.
    BadSignature,
    /// No update was found by a check before the install.
    NoUpdate,
    /// This system cannot install updates from within the app.
    Unsupported,
    /// Unexpected failure inside the app.
    Internal,
}

impl From<&tauri_plugin_updater::Error> for UpdateError {
    fn from(err: &tauri_plugin_updater::Error) -> Self {
        use tauri_plugin_updater::Error as E;
        // The plugin's error is `non_exhaustive`, so new kinds end up as `Internal`.
        if matches!(
            err,
            E::Minisign(_) | E::Base64(_) | E::SignatureUtf8(_) | E::SignedVersionMismatch { .. }
        ) {
            Self::BadSignature
        } else if matches!(
            err,
            E::Reqwest(_)
                | E::Network(_)
                | E::ReleaseNotFound
                | E::Serialization(_)
                | E::Semver(_)
                | E::UrlParse(_)
                | E::Http(_)
                | E::TargetNotFound(_)
                | E::TargetsNotFound(_)
        ) {
            Self::Network
        } else if matches!(
            err,
            E::UnsupportedOs | E::UnsupportedArch | E::InvalidUpdaterFormat
        ) {
            Self::Unsupported
        } else {
            Self::Internal
        }
    }
}

/// `err` as its code, logged with `what` failed.
pub fn logged(what: &str, err: &tauri_plugin_updater::Error) -> UpdateError {
    log::warn!("cannot {what}: {err}");
    UpdateError::from(err)
}

#[cfg(test)]
mod tests {
    use tauri_plugin_updater::Error as E;

    use super::*;

    #[test]
    fn unreachable_github_is_a_network_error() {
        assert_eq!(
            UpdateError::from(&E::Network("timed out".into())),
            UpdateError::Network
        );
        assert_eq!(UpdateError::from(&E::ReleaseNotFound), UpdateError::Network);
        assert_eq!(
            UpdateError::from(&E::TargetNotFound("windows-x86_64".into())),
            UpdateError::Network
        );
    }

    #[test]
    fn a_wrong_signature_is_reported_as_such() {
        assert_eq!(
            UpdateError::from(&E::SignatureUtf8("garbage".into())),
            UpdateError::BadSignature
        );
        assert_eq!(
            UpdateError::from(&E::SignedVersionMismatch {
                signed: "0.14.0".into(),
                announced: "0.15.0".into(),
            }),
            UpdateError::BadSignature
        );
    }

    #[test]
    fn an_unsupported_system_is_reported_as_such() {
        assert_eq!(
            UpdateError::from(&E::UnsupportedOs),
            UpdateError::Unsupported
        );
    }

    #[test]
    fn other_failures_are_internal() {
        assert_eq!(UpdateError::from(&E::EmptyEndpoints), UpdateError::Internal);
        assert_eq!(
            UpdateError::from(&E::Io(std::io::Error::other("disk full"))),
            UpdateError::Internal
        );
    }

    #[test]
    fn errors_are_sent_as_codes() {
        assert_eq!(
            serde_json::to_value(UpdateError::BadSignature).ok(),
            Some(serde_json::json!({ "code": "badSignature" }))
        );
    }
}
