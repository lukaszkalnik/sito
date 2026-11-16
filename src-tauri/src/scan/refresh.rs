//! Rescanning the open folder in the background: after it was shown from
//! the photo cache, or on demand („Odśwież”).

use std::path::PathBuf;

use sito_core::scan::ScanError;
use sito_core::timeline::CachedScan;
use tauri::{AppHandle, Emitter, Manager, State};

use super::photo_cache::PhotoCacheState;
use super::state::{AppState, Generation};
use super::{SCAN_REFRESHED, ScanSummaryDto, completion};
use crate::accept;
use crate::missing::{self, Vanished};
use crate::reject::TIMELINE_CHANGED;
use crate::thumbnails::Thumbnails;

/// Event with the scan id when a background refresh of it starts.
pub const SCAN_REFRESHING: &str = "scan-refreshing";

/// Rescans the open folder in the background, as when it is reopened:
/// stops its EXIF pass (the refresh restarts it at its end), announces
/// [`SCAN_REFRESHING`] and ends with [`SCAN_REFRESHED`]. Does nothing
/// without an open folder.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn refresh_folder(app: AppHandle, state: State<'_, AppState>) {
    let Some((root, scan_id)) = state
        .scan()
        .as_ref()
        .map(|scan| (scan.timeline.root.clone(), scan.id))
    else {
        return;
    };
    let generation = state.begin_refresh();
    if let Err(err) = app.emit(SCAN_REFRESHING, scan_id) {
        log::warn!("cannot announce the refresh: {err}");
    }
    refresh_in_background(app, root, scan_id, generation);
}

/// Rescans `path` (shown as scan `scan_id`) and swaps the result in,
/// keeping photo ids, also of photos moved or renamed outside Sito. Stops,
/// keeping what it read in the cache, once another folder is opened (or a
/// new refresh started) after `generation`. If the folder is gone, the
/// shown timeline stays and the windows are told ([`missing::FOLDER_MISSING`]).
pub(super) fn refresh_in_background(
    app: AppHandle,
    path: PathBuf,
    scan_id: u64,
    generation: Generation,
) {
    tauri::async_runtime::spawn_blocking(move || {
        let should_stop = || !app.state::<AppState>().is_latest(generation);
        let result = app
            .state::<PhotoCacheState>()
            .scan(&path, should_stop, |_| {});
        let folder_gone = matches!(result, Err(ScanError::NotFound(_)));
        let summary = match result {
            Err(ScanError::Stopped) => {
                log::info!("background rescan of {} stopped", path.display());
                return;
            }
            Ok(scanned) => swap_in(&app, scan_id, scanned),
            Err(err) => {
                // The cached view stays; it may be out of date.
                log::warn!("background rescan of {} failed: {err}", path.display());
                shown_summary(&app, scan_id)
            }
        };
        let Some(summary) = summary else {
            // Another folder was opened meanwhile.
            return;
        };
        if let Err(err) = app.emit(SCAN_REFRESHED, summary) {
            log::warn!("cannot announce the refreshed summary: {err}");
        }
        if folder_gone {
            missing::announce(&app, scan_id, Vanished::Folder);
            return;
        }
        app.state::<PhotoCacheState>()
            .clean_thumbnails(&app.state::<Thumbnails>());
        completion::complete_in_background(app, path, scan_id, generation);
    });
}

/// Replaces scan `scan_id` with the rescan `scanned` and returns its
/// summary; `None` if another folder was opened meanwhile.
fn swap_in(app: &AppHandle, scan_id: u64, scanned: CachedScan) -> Option<ScanSummaryDto> {
    accept::follow_moves(app, &scanned.moves);
    let summary = ScanSummaryDto::new(scan_id, &scanned.timeline);
    if !app
        .state::<AppState>()
        .refresh(scan_id, scanned.timeline, &scanned.moves)
    {
        return None;
    }
    if let Err(err) = app.emit(TIMELINE_CHANGED, ()) {
        log::warn!("cannot announce the refreshed timeline: {err}");
    }
    Some(summary)
}

/// The summary of scan `scan_id` as shown; `None` if it is not shown.
fn shown_summary(app: &AppHandle, scan_id: u64) -> Option<ScanSummaryDto> {
    let state = app.state::<AppState>();
    let scan = state.scan();
    let scan = scan.as_ref().filter(|scan| scan.id == scan_id)?;
    Some(ScanSummaryDto::new(scan_id, &scan.timeline))
}
