//! Series commands: group the current timeline into series.

use std::time::Duration;

use serde::Serialize;
use sito_core::series::{self, DEFAULT_MAX_GAP, Series};
use tauri::State;

use crate::state::AppState;

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
/// default gap.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn get_series(max_gap_seconds: Option<u32>, state: State<'_, AppState>) -> Vec<SeriesDto> {
    let max_gap = max_gap_seconds.map_or(DEFAULT_MAX_GAP, |s| Duration::from_secs(s.into()));
    state.scan().as_ref().map_or_else(Vec::new, |scan| {
        series::find_time_series(&scan.timeline.photos, max_gap)
            .into_iter()
            .map(SeriesDto::from)
            .collect()
    })
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
