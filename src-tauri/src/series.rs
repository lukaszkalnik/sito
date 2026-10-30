//! Series commands: group the current timeline into series.

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use sito_core::series::{DEFAULT_MAX_GAP, Grouping, SeriesLabel, SeriesRange};
use tauri::{AppHandle, Manager};

use crate::error::{CommandError, off_main_thread};
use crate::scan::state::AppState;

/// The series as sent to the UI, with the order the photos are shown in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GroupingDto {
    /// Timeline positions (indices of the timeline entries) in display order.
    order: Vec<usize>,
    /// The series as ranges of display positions, sorted by start.
    series: Vec<SeriesDto>,
}

/// One series as sent to the UI: a range of display positions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDto {
    /// Display position of the series' first (newest) photo.
    start: usize,
    /// Number of photos in the series.
    len: usize,
    /// What the header says about the camera.
    label: SeriesLabelDto,
}

/// Mirrors [`SeriesLabel`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SeriesLabelDto {
    /// Nothing about the camera.
    Plain,
    /// The camera's name.
    Camera {
        /// Camera description, e.g. `Canon EOS R6`.
        name: String,
    },
    /// That the camera is unknown.
    UnknownCamera,
}

impl From<&SeriesLabel> for SeriesLabelDto {
    fn from(label: &SeriesLabel) -> Self {
        match label {
            SeriesLabel::Plain => Self::Plain,
            SeriesLabel::Camera(name) => Self::Camera { name: name.clone() },
            SeriesLabel::UnknownCamera => Self::UnknownCamera,
        }
    }
}

impl From<&Grouping> for GroupingDto {
    fn from(grouping: &Grouping) -> Self {
        // Both lists are sorted by the series' newest photo.
        let series = grouping
            .arrangement
            .ranges()
            .iter()
            .zip(&grouping.series)
            .map(|(&SeriesRange { start, len }, s)| SeriesDto {
                start,
                len,
                label: SeriesLabelDto::from(&s.label),
            })
            .collect();
        Self {
            order: grouping.arrangement.order().to_vec(),
            series,
        }
    }
}

/// Returns the series of the last finished scan (none without a scan) and
/// the order to show its photos in. A camera's photos at most
/// `max_gap_seconds` apart form a series; `None` uses the default gap. Found
/// off the main thread, as for the timeline.
#[tauri::command]
pub async fn get_series(
    max_gap_seconds: Option<u32>,
    app: AppHandle,
) -> Result<GroupingDto, CommandError> {
    let max_gap = max_gap_seconds.map_or(DEFAULT_MAX_GAP, |s| Duration::from_secs(s.into()));
    off_main_thread(move || {
        cached_series(&app.state::<AppState>(), max_gap)
            .map(|grouping| GroupingDto::from(grouping.as_ref()))
            .unwrap_or_default()
    })
    .await
}

/// The series of the last finished scan for `max_gap` (`None` without a
/// scan), found once per change of the timeline and kept in the scan. The
/// photos are copied under the lock and grouped after it, so thumbnails
/// (which look photos up in the scan) are not held up.
pub(crate) fn cached_series(state: &AppState, max_gap: Duration) -> Option<Arc<Grouping>> {
    let (scan_id, revision, all, removed) = {
        let scan = state.scan();
        let scan = scan.as_ref()?;
        if let Some(grouping) = scan.cached_series(max_gap) {
            return Some(grouping);
        }
        let (all, removed) = scan.photos_with_removed();
        (scan.id, scan.revision(), all, removed)
    };
    let grouping = Arc::new(Grouping::new(&all, &removed, max_gap));
    if let Some(scan) = state.scan().as_mut().filter(|scan| scan.id == scan_id) {
        scan.store_series(revision, max_gap, Arc::clone(&grouping));
    }
    Some(grouping)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;
    use sito_core::series::Series;
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
    fn grouping_serializes_the_order_and_ranges_with_labels() {
        let series = [
            Series {
                members: vec![0, 2],
                label: SeriesLabel::Camera("Pixel 8".to_owned()),
            },
            Series {
                members: vec![1],
                label: SeriesLabel::UnknownCamera,
            },
        ];
        let grouping = Grouping {
            arrangement: sito_core::series::Arrangement::new(4, &series),
            series: series.to_vec(),
        };

        assert_eq!(
            serde_json::to_value(GroupingDto::from(&grouping)).unwrap(),
            json!({
                "order": [0, 2, 1, 3],
                "series": [
                    { "start": 0, "len": 2, "label": { "kind": "camera", "name": "Pixel 8" } },
                    { "start": 2, "len": 1, "label": { "kind": "unknownCamera" } },
                ],
            })
        );
    }

    #[test]
    fn a_plain_label_serializes_as_its_kind() {
        assert_eq!(
            serde_json::to_value(SeriesLabelDto::from(&SeriesLabel::Plain)).unwrap(),
            json!({ "kind": "plain" })
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
        assert_eq!(first.series.len(), 1);
        assert!(Arc::ptr_eq(&first, &again));

        let position = first.series[0].members[0];
        let id = state.scan().as_ref().unwrap().entry_at(position).unwrap().0;
        state.scan().as_mut().unwrap().remove(id);
        let after = cached_series(&state, DEFAULT_MAX_GAP).unwrap();
        assert!(!Arc::ptr_eq(&first, &after));
        assert_eq!(
            after.series[0].members.len(),
            first.series[0].members.len() - 1
        );
    }
}
