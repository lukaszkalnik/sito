//! Series commands: group the current timeline into series.

use std::time::Duration;

use serde::Serialize;
use sito_core::series::{self, DEFAULT_MAX_GAP, Series};
use sito_core::timeline::Photo;
use tauri::{AppHandle, Manager};

use crate::error::{CommandError, off_main_thread};
use crate::scan::state::{AppState, Scan};

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
        // Copied under the lock, grouped after it.
        let Some((all, removed)) = app
            .state::<AppState>()
            .scan()
            .as_ref()
            .map(Scan::photos_with_removed)
        else {
            return Vec::new();
        };
        series_with_removed(&all, &removed, max_gap)
            .into_iter()
            .map(SeriesDto::from)
            .collect()
    })
    .await
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
    use serde_json::json;

    use super::*;

    #[test]
    fn series_serializes_as_a_range() {
        assert_eq!(
            serde_json::to_value(SeriesDto::from(Series { start: 4, len: 3 })).unwrap(),
            json!({ "start": 4, "len": 3 })
        );
    }
}
