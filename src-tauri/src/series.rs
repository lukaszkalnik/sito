//! Series commands: group the current timeline into series.

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use sito_core::series::{self, DEFAULT_MAX_GAP, Series};
use sito_core::timeline::Photo;
use tauri::{AppHandle, Manager};

use crate::error::{CommandError, off_main_thread};
use crate::scan::state::AppState;

/// One series as sent to the UI: a range of timeline entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDto {
    /// Id of the series' first (newest) timeline entry.
    start: usize,
    /// Number of photos in the series.
    len: usize,
}

impl From<Series> for SeriesDto {
    fn from(s: Series) -> Self {
        Self {
            start: s.start,
            len: s.len,
        }
    }
}

/// Returns the series of the last finished scan (empty if none). Neighbouring
/// photos at most `max_gap_seconds` apart form a series; `None` uses the
/// default gap. Found off the main thread, as for the timeline.
#[tauri::command]
pub async fn get_series(
    max_gap_seconds: Option<u32>,
    app: AppHandle,
) -> Result<Vec<SeriesDto>, CommandError> {
    let max_gap = max_gap_seconds.map_or(DEFAULT_MAX_GAP, |s| Duration::from_secs(s.into()));
    off_main_thread(move || {
        cached_series(&app.state::<AppState>(), max_gap)
            .map(|series| series.iter().copied().map(SeriesDto::from).collect())
            .unwrap_or_default()
    })
    .await
}

/// The series of the last finished scan for `max_gap` (`None` without a
/// scan), found once per change of the timeline and kept in the scan. The
/// photos are copied under the lock and grouped after it, so thumbnails
/// (which look photos up in the scan) are not held up.
pub(crate) fn cached_series(state: &AppState, max_gap: Duration) -> Option<Arc<Vec<Series>>> {
    let (scan_id, revision, all, removed) = {
        let scan = state.scan();
        let scan = scan.as_ref()?;
        if let Some(series) = scan.cached_series(max_gap) {
            return Some(series);
        }
        let (all, removed) = scan.photos_with_removed();
        (scan.id, scan.revision(), all, removed)
    };
    let series = Arc::new(series_with_removed(&all, &removed, max_gap));
    if let Some(scan) = state.scan().as_mut().filter(|scan| scan.id == scan_id) {
        scan.store_series(revision, max_gap, Arc::clone(&series));
    }
    Some(series)
}

/// The series of `all` photos (newest first), leaving out the `removed` ones.
/// Rejected photos still shape them, so a series stays one while any of its
/// photos is left.
pub(crate) fn series_with_removed(
    all: &[Photo],
    removed: &[bool],
    max_gap: Duration,
) -> Vec<Series> {
    series::without_removed(&series::find_time_series(all, max_gap), removed)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;
    use sito_core::timeline::build_timeline;

    use super::*;

    /// App state with a scan of the EXIF fixtures (two of them form a series).
    fn scanned() -> AppState {
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/exif");
        let state = AppState::default();
        state.set_timeline(build_timeline(&fixtures, |_| {}).unwrap());
        state
    }

    #[test]
    fn series_serializes_as_a_range() {
        assert_eq!(
            serde_json::to_value(SeriesDto::from(Series { start: 4, len: 3 })).unwrap(),
            json!({ "start": 4, "len": 3 })
        );
    }

    #[test]
    fn no_series_without_a_scan() {
        assert!(cached_series(&AppState::default(), DEFAULT_MAX_GAP).is_none());
    }

    #[test]
    fn series_are_found_once_until_the_timeline_changes() {
        let state = scanned();

        let first = cached_series(&state, DEFAULT_MAX_GAP).unwrap();
        let again = cached_series(&state, DEFAULT_MAX_GAP).unwrap();
        assert_eq!(first.len(), 1);
        assert!(Arc::ptr_eq(&first, &again));

        let start = first[0].start;
        let id = state.scan().as_ref().unwrap().entry_at(start).unwrap().0;
        state.scan().as_mut().unwrap().remove(id);
        let after = cached_series(&state, DEFAULT_MAX_GAP).unwrap();
        assert!(!Arc::ptr_eq(&first, &after));
        assert_eq!(after[0].len, first[0].len - 1);
    }
}
