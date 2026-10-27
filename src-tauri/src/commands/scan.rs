//! Folder scan commands: build the timeline and serve it to the UI.

use std::path::{Path, PathBuf};

use serde::Serialize;
use sito_core::scan::ScanError;
use sito_core::timeline::{self, Photo, TimeSource, Timeline, TimelineProgress};
use tauri::State;
use tauri::ipc::Channel;

use crate::state::AppState;

/// Progress update sent to the UI while a scan is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(
    tag = "phase",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ScanProgressDto {
    /// Searching folders for photos.
    Searching {
        /// Photos found so far.
        found: u64,
    },
    /// Reading capture times.
    Reading {
        /// Photos read so far.
        done: u64,
        /// Photos to read in total.
        total: u64,
    },
}

impl From<TimelineProgress> for ScanProgressDto {
    fn from(p: TimelineProgress) -> Self {
        match p {
            TimelineProgress::Searching { found } => Self::Searching { found },
            TimelineProgress::Reading { done, total } => Self::Reading { done, total },
        }
    }
}

/// Result of a finished scan, as sent to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummaryDto {
    /// Identifies this scan; part of thumbnail URLs.
    scan_id: u64,
    photos_found: usize,
    unreadable_entries: u64,
    without_exif_date: usize,
}

impl ScanSummaryDto {
    fn new(scan_id: u64, t: &Timeline) -> Self {
        Self {
            scan_id,
            photos_found: t.photos.len(),
            unreadable_entries: t.unreadable_entries,
            without_exif_date: t.without_exif_date(),
        }
    }
}

/// One photo on the timeline, as sent to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntryDto {
    /// Stable id of the photo within its scan (kept when other photos are
    /// rejected or restored).
    id: usize,
    /// File name.
    name: String,
    /// Folder of the file relative to the scanned folder (empty for the root).
    relative_dir: String,
    /// File path relative to the scanned folder, with the OS path separator.
    relative_path: String,
    /// Capture time as local ISO 8601 without offset, e.g. `2024-06-01T12:00:05`.
    taken_at: String,
    /// `true` if the time comes from the file date instead of EXIF.
    from_file_date: bool,
}

impl TimelineEntryDto {
    fn new(id: usize, photo: &Photo, root: &Path) -> Self {
        let relative = photo.path.strip_prefix(root).unwrap_or(&photo.path);
        let relative_dir = relative
            .parent()
            .map(|dir| dir.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            id,
            name: photo
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            relative_dir,
            relative_path: relative.to_string_lossy().into_owned(),
            taken_at: photo.taken_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
            from_file_date: photo.time_source == TimeSource::FileModified,
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
    /// Undo would overwrite a file that now has the photo's original name.
    Occupied,
    /// The rejected photo is no longer in the rejected folder.
    Missing,
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

/// Builds the timeline of `path`, sending progress through `on_progress`, and
/// keeps it for [`get_timeline`].
///
/// The scan runs on a blocking thread so the UI stays responsive.
#[tauri::command]
pub async fn start_scan(
    path: PathBuf,
    on_progress: Channel<ScanProgressDto>,
    state: State<'_, AppState>,
) -> Result<ScanSummaryDto, CommandError> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        timeline::build_timeline(&path, |progress| {
            // A closed channel only means the UI stopped listening.
            let _ = on_progress.send(progress.into());
        })
    })
    .await;

    match result {
        Ok(Ok(timeline)) => {
            let mut summary = ScanSummaryDto::new(0, &timeline);
            summary.scan_id = state.set_timeline(timeline);
            Ok(summary)
        }
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

/// Returns the timeline of the last finished scan (empty if none).
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn get_timeline(state: State<'_, AppState>) -> Vec<TimelineEntryDto> {
    state.scan().as_ref().map_or_else(Vec::new, |scan| {
        scan.photos()
            .map(|(id, photo)| TimelineEntryDto::new(id, photo, &scan.timeline.root))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::NaiveDate;
    use serde_json::json;
    use sito_core::metadata::Orientation;

    use super::*;

    fn photo(path: &str, source: TimeSource) -> Photo {
        Photo {
            path: PathBuf::from(path),
            size: 1,
            modified: None,
            taken_at: NaiveDate::from_ymd_opt(2024, 6, 1)
                .unwrap()
                .and_hms_opt(12, 0, 5)
                .unwrap(),
            time_source: source,
            orientation: Orientation::NORMAL,
            camera: None,
        }
    }

    #[test]
    fn timeline_entry_serializes_in_camel_case() {
        let root = PathBuf::from("gallery");
        let p = photo(
            &format!("gallery{0}Camera{0}IMG_1.jpg", std::path::MAIN_SEPARATOR),
            TimeSource::Exif,
        );

        assert_eq!(
            serde_json::to_value(TimelineEntryDto::new(3, &p, &root)).unwrap(),
            json!({
                "id": 3,
                "name": "IMG_1.jpg",
                "relativeDir": "Camera",
                "relativePath": format!("Camera{}IMG_1.jpg", std::path::MAIN_SEPARATOR),
                "takenAt": "2024-06-01T12:00:05",
                "fromFileDate": false
            })
        );
    }

    #[test]
    fn nested_photo_has_multi_level_relative_paths() {
        let root = PathBuf::from("gallery");
        let p = photo(
            &["gallery", "DCIM", "Camera", "2024", "06", "IMG_3.jpg"]
                .join(std::path::MAIN_SEPARATOR_STR),
            TimeSource::Exif,
        );

        let dto = TimelineEntryDto::new(0, &p, &root);

        let sep = std::path::MAIN_SEPARATOR_STR;
        assert_eq!(dto.relative_dir, ["DCIM", "Camera", "2024", "06"].join(sep));
        assert_eq!(
            dto.relative_path,
            ["DCIM", "Camera", "2024", "06", "IMG_3.jpg"].join(sep)
        );
        assert_eq!(dto.name, "IMG_3.jpg");
    }

    #[test]
    fn file_in_root_has_empty_relative_dir_and_file_date_flag() {
        let root = PathBuf::from("gallery");
        let p = photo(
            &format!("gallery{}IMG_2.jpg", std::path::MAIN_SEPARATOR),
            TimeSource::FileModified,
        );

        let dto = TimelineEntryDto::new(0, &p, &root);

        assert_eq!(dto.relative_dir, "");
        assert_eq!(dto.relative_path, "IMG_2.jpg");
        assert!(dto.from_file_date);
    }

    #[test]
    fn summary_serializes_in_camel_case() {
        let timeline = Timeline {
            root: PathBuf::from("gallery"),
            photos: vec![
                photo("a.jpg", TimeSource::Exif),
                photo("b.jpg", TimeSource::FileModified),
            ],
            unreadable_entries: 1,
        };

        assert_eq!(
            serde_json::to_value(ScanSummaryDto::new(7, &timeline)).unwrap(),
            json!({ "scanId": 7, "photosFound": 2, "unreadableEntries": 1, "withoutExifDate": 1 })
        );
    }

    #[test]
    fn progress_serializes_with_phase_tag() {
        assert_eq!(
            serde_json::to_value(ScanProgressDto::from(TimelineProgress::Searching {
                found: 42
            }))
            .unwrap(),
            json!({ "phase": "searching", "found": 42 })
        );
        assert_eq!(
            serde_json::to_value(ScanProgressDto::from(TimelineProgress::Reading {
                done: 200,
                total: 1000
            }))
            .unwrap(),
            json!({ "phase": "reading", "done": 200, "total": 1000 })
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
