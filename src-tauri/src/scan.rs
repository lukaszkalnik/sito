//! Folder scan commands: build the timeline and serve it to the UI.

use std::path::{Path, PathBuf};

use serde::Serialize;
use sito_core::scan::ScanError;
use sito_core::takeout::TakeoutUse;
use sito_core::timeline::{Photo, TimeSource, Timeline, TimelineProgress};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::CommandError;
use crate::reject::TIMELINE_CHANGED;
use crate::thumbnails::Thumbnails;
use photo_cache::PhotoCacheState;
use state::AppState;

pub mod photo_cache;
pub mod state;

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
    without_date: usize,
    /// Shown from the photo cache; a background rescan is running and ends
    /// with a `scan-refreshed` event.
    refreshing: bool,
}

impl ScanSummaryDto {
    fn new(scan_id: u64, t: &Timeline) -> Self {
        Self {
            scan_id,
            photos_found: t.photos.len(),
            unreadable_entries: t.unreadable_entries,
            without_date: t.without_date(),
            refreshing: false,
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
    /// Where the capture time comes from.
    time_source: TimeSourceDto,
}

/// Where a photo's capture time comes from, as sent to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TimeSourceDto {
    /// EXIF capture time.
    Exif,
    /// The Google Takeout date (the photo has no usable EXIF time).
    Takeout,
    /// The Takeout date, which replaces an EXIF time that differs: the date
    /// was corrected in Google Photos.
    TakeoutCorrected,
    /// The file's modification time (no usable capture time).
    File,
}

impl From<TimeSource> for TimeSourceDto {
    fn from(source: TimeSource) -> Self {
        match source {
            TimeSource::Exif => Self::Exif,
            TimeSource::Takeout(TakeoutUse::MissingExif | TakeoutUse::ImplausibleExif) => {
                Self::Takeout
            }
            TimeSource::Takeout(TakeoutUse::Corrected) => Self::TakeoutCorrected,
            TimeSource::FileModified => Self::File,
        }
    }
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
            time_source: photo.time_source.into(),
        }
    }
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

/// Event with the [`ScanSummaryDto`] of a finished background refresh.
pub const SCAN_REFRESHED: &str = "scan-refreshed";

/// Opens the folder `path` and keeps its timeline for [`get_timeline`].
///
/// A folder scanned before is shown at once from the photo cache; it is then
/// rescanned in the background (the summary says `refreshing`), and the
/// result replaces the timeline with [`TIMELINE_CHANGED`] and
/// [`SCAN_REFRESHED`]. Otherwise the folder is scanned now, sending progress
/// through `on_progress`. Either way only new or changed photos are read.
#[tauri::command]
pub async fn start_scan(
    path: PathBuf,
    on_progress: Channel<ScanProgressDto>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ScanSummaryDto, CommandError> {
    if path.is_dir()
        && let Some(cached) = app.state::<PhotoCacheState>().cached_timeline(&path)
    {
        let mut summary = ScanSummaryDto::new(0, &cached);
        summary.scan_id = state.set_timeline(cached);
        summary.refreshing = true;
        refresh_in_background(app, path, summary.scan_id);
        return Ok(summary);
    }

    let scanner = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        scanner.state::<PhotoCacheState>().scan(&path, |progress| {
            // A closed channel only means the UI stopped listening.
            let _ = on_progress.send(progress.into());
        })
    })
    .await;

    match result {
        Ok(Ok(timeline)) => {
            let mut summary = ScanSummaryDto::new(0, &timeline);
            summary.scan_id = state.set_timeline(timeline);
            clean_thumbnails_in_background(app);
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

/// Rescans `path` (shown from the cache as scan `scan_id`) and swaps the
/// result in, keeping photo ids.
fn refresh_in_background(app: AppHandle, path: PathBuf, scan_id: u64) {
    tauri::async_runtime::spawn_blocking(move || {
        let summary = match app.state::<PhotoCacheState>().scan(&path, |_| {}) {
            Ok(timeline) => {
                let summary = ScanSummaryDto::new(scan_id, &timeline);
                if !app.state::<AppState>().refresh(scan_id, timeline) {
                    // Another folder was opened meanwhile.
                    return;
                }
                if let Err(err) = app.emit(TIMELINE_CHANGED, ()) {
                    log::warn!("cannot announce the refreshed timeline: {err}");
                }
                summary
            }
            Err(err) => {
                // The cached view stays; it may be out of date.
                log::warn!("background rescan of {} failed: {err}", path.display());
                let state = app.state::<AppState>();
                let scan = state.scan();
                match scan.as_ref() {
                    Some(scan) if scan.id == scan_id => {
                        ScanSummaryDto::new(scan_id, &scan.timeline)
                    }
                    _ => return,
                }
            }
        };
        if let Err(err) = app.emit(SCAN_REFRESHED, summary) {
            log::warn!("cannot announce the refreshed summary: {err}");
        }
        app.state::<PhotoCacheState>()
            .clean_thumbnails(&app.state::<Thumbnails>());
    });
}

fn clean_thumbnails_in_background(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<PhotoCacheState>()
            .clean_thumbnails(&app.state::<Thumbnails>());
    });
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
            sidecar: None,
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
                "timeSource": "exif"
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
    fn file_in_root_has_empty_relative_dir_and_file_time_source() {
        let root = PathBuf::from("gallery");
        let p = photo(
            &format!("gallery{}IMG_2.jpg", std::path::MAIN_SEPARATOR),
            TimeSource::FileModified,
        );

        let dto = TimelineEntryDto::new(0, &p, &root);

        assert_eq!(dto.relative_dir, "");
        assert_eq!(dto.relative_path, "IMG_2.jpg");
        assert_eq!(dto.time_source, TimeSourceDto::File);
    }

    #[test]
    fn takeout_times_tell_corrected_dates_apart() {
        let source = |s| serde_json::to_value(TimeSourceDto::from(s)).unwrap();
        assert_eq!(
            source(TimeSource::Takeout(TakeoutUse::MissingExif)),
            json!("takeout")
        );
        assert_eq!(
            source(TimeSource::Takeout(TakeoutUse::ImplausibleExif)),
            json!("takeout")
        );
        assert_eq!(
            source(TimeSource::Takeout(TakeoutUse::Corrected)),
            json!("takeoutCorrected")
        );
    }

    #[test]
    fn summary_serializes_in_camel_case() {
        let timeline = Timeline {
            root: PathBuf::from("gallery"),
            photos: vec![
                photo("a.jpg", TimeSource::Exif),
                photo("b.jpg", TimeSource::FileModified),
            ],
            rejected: vec![],
            unreadable_entries: 1,
        };

        assert_eq!(
            serde_json::to_value(ScanSummaryDto::new(7, &timeline)).unwrap(),
            json!({ "scanId": 7, "photosFound": 2, "unreadableEntries": 1, "withoutDate": 1, "refreshing": false })
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
