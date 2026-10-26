//! Folder scan command: counts photos and streams progress to the UI.

use std::path::PathBuf;

use serde::Serialize;
use sito_core::scan::{self, ScanError, ScanProgress, ScanSummary};
use tauri::ipc::Channel;

/// Progress update sent to the UI while a scan is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressDto {
    entries_seen: u64,
    photos_found: u64,
}

impl From<ScanProgress> for ScanProgressDto {
    fn from(p: ScanProgress) -> Self {
        Self {
            entries_seen: p.entries_seen,
            photos_found: p.photos_found,
        }
    }
}

/// Result of a finished scan, as sent to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummaryDto {
    photos_found: u64,
    entries_seen: u64,
    unreadable_entries: u64,
}

impl From<ScanSummary> for ScanSummaryDto {
    fn from(s: ScanSummary) -> Self {
        Self {
            photos_found: s.photos_found,
            entries_seen: s.entries_seen,
            unreadable_entries: s.unreadable_entries,
        }
    }
}

/// Error sent to the UI. Only a code is sent; the UI shows a translated message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "camelCase")]
pub enum CommandError {
    /// The folder does not exist.
    NotFound,
    /// The path is not a folder.
    NotADirectory,
    /// The folder could not be read.
    Io,
    /// Unexpected failure inside the app.
    Internal,
}

impl From<ScanError> for CommandError {
    fn from(err: ScanError) -> Self {
        match err {
            ScanError::NotFound(_) => Self::NotFound,
            ScanError::NotADirectory(_) => Self::NotADirectory,
            ScanError::Io { .. } => Self::Io,
        }
    }
}

/// Counts the photos under `path`, sending progress through `on_progress`.
///
/// The scan runs on a blocking thread so the UI stays responsive.
#[tauri::command]
pub async fn start_scan(
    path: PathBuf,
    on_progress: Channel<ScanProgressDto>,
) -> Result<ScanSummaryDto, CommandError> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        scan::scan(&path, |progress| {
            // A closed channel only means the UI stopped listening.
            let _ = on_progress.send(progress.into());
        })
    })
    .await;

    match result {
        Ok(Ok(summary)) => Ok(summary.into()),
        Ok(Err(err)) => {
            log::warn!("scan failed: {err}");
            Err(err.into())
        }
        Err(err) => {
            log::error!("scan task failed: {err}");
            Err(CommandError::Internal)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;

    #[test]
    fn summary_serializes_in_camel_case() {
        let dto = ScanSummaryDto::from(ScanSummary {
            photos_found: 3,
            entries_seen: 10,
            unreadable_entries: 1,
        });

        assert_eq!(
            serde_json::to_value(dto).unwrap(),
            json!({ "photosFound": 3, "entriesSeen": 10, "unreadableEntries": 1 })
        );
    }

    #[test]
    fn progress_serializes_in_camel_case() {
        let dto = ScanProgressDto::from(ScanProgress {
            entries_seen: 500,
            photos_found: 42,
        });

        assert_eq!(
            serde_json::to_value(dto).unwrap(),
            json!({ "entriesSeen": 500, "photosFound": 42 })
        );
    }

    #[test]
    fn scan_errors_map_to_codes() {
        let cases = [
            (ScanError::NotFound(PathBuf::new()), "notFound"),
            (ScanError::NotADirectory(PathBuf::new()), "notADirectory"),
            (
                ScanError::Io {
                    path: PathBuf::new(),
                    source: std::io::Error::other("boom"),
                },
                "io",
            ),
        ];

        for (err, code) in cases {
            let value = serde_json::to_value(CommandError::from(err)).unwrap();
            assert_eq!(value, json!({ "code": code }));
        }
    }

    #[test]
    fn internal_error_has_code() {
        assert_eq!(
            serde_json::to_value(CommandError::Internal).unwrap(),
            json!({ "code": "internal" })
        );
    }
}
