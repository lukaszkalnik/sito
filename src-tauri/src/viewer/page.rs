//! What the viewer shows for one photo: the photo, its series and the way on.

use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use sito_core::dimensions::{Dimensions, read_dimensions};
use sito_core::navigation::{self, View};
use sito_core::series::Series;
use sito_core::timeline::Photo;
use tauri::{AppHandle, Manager};

use super::{ViewMode, ViewerRequest};
use crate::error::{CommandError, off_main_thread};
use crate::scan::TimelineEntryDto;
use crate::scan::state::{AppState, Scan};
use crate::series::cached_series;

impl From<ViewMode> for View {
    fn from(mode: ViewMode) -> Self {
        match mode {
            ViewMode::Series => Self::Series,
            ViewMode::All => Self::All,
        }
    }
}

/// One photo as the viewer shows it: where it is and what it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerPhotoDto {
    /// The photo, its series and the ids the viewer can go on to.
    #[serde(flatten)]
    page: Page,
    /// File size and resolution of the photo.
    details: PhotoDetailsDto,
}

/// A photo with its series and the ids it can go on to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    /// The photo.
    entry: TimelineEntryDto,
    /// Id of the previous photo of the view, if any.
    previous: Option<usize>,
    /// Id of the next photo of the view, if any.
    next: Option<usize>,
    /// The photos of its series, newest first (empty for a single photo).
    series: Vec<TimelineEntryDto>,
    /// Id of the photo to show once this one is rejected, if any is left.
    after_reject: Option<usize>,
}

/// File size and resolution of the shown photo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PhotoDetailsDto {
    /// Size of the photo file in bytes.
    size_bytes: u64,
    /// Width and height as shown (after the EXIF orientation); `None` for RAW
    /// files and files whose header cannot be read.
    resolution: Option<ResolutionDto>,
}

/// Width and height in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolutionDto {
    width: u32,
    height: u32,
}

impl From<Dimensions> for ResolutionDto {
    fn from(dimensions: Dimensions) -> Self {
        Self {
            width: dimensions.width,
            height: dimensions.height,
        }
    }
}

impl PhotoDetailsDto {
    /// The details of `photo`; the resolution is read from its file.
    fn of(photo: &Photo) -> Self {
        Self {
            size_bytes: photo.size,
            resolution: read_dimensions(&photo.path)
                .map(|dimensions| dimensions.upright(photo.orientation()).into()),
        }
    }
}

/// What the viewer shows for `request`: `None` if the photo is not on the
/// timeline of the request's scan (another scan replaced it, or a restored
/// photo is not back yet).
#[tauri::command]
pub async fn get_viewer_photo(
    request: ViewerRequest,
    app: AppHandle,
) -> Result<Option<ViewerPhotoDto>, CommandError> {
    off_main_thread(move || viewer_photo(&app.state::<AppState>(), request)).await
}

/// See [`get_viewer_photo`].
pub(crate) fn viewer_photo(state: &AppState, request: ViewerRequest) -> Option<ViewerPhotoDto> {
    let series = cached_series(state, Duration::from_secs(request.gap.into()))?;
    let (page, photo) = {
        let scan = state.scan();
        let scan = scan.as_ref().filter(|scan| scan.id == request.scan_id)?;
        page(scan, &series, request)?
    };
    // Read from the file once the scan is unlocked, so thumbnails never wait.
    let details = PhotoDetailsDto::of(&photo);
    Some(ViewerPhotoDto { page, details })
}

/// The page of `request.id` in `scan`, whose series are `series`, and the photo.
fn page(scan: &Scan, series: &[Series], request: ViewerRequest) -> Option<(Page, Photo)> {
    let position = scan.position_of(request.id)?;
    let root = scan.timeline.root.as_path();
    let count = scan.photo_count();
    let view = View::from(request.mode);
    let around = navigation::neighbours(count, series, view, position);
    let id_at = |position: Option<usize>| Some(scan.entry_at(position?)?.0);
    let members = navigation::series_of(series, position)
        .map_or(0..0, |s| s.start..s.start + s.len)
        .filter_map(|member| entry(scan, root, member))
        .collect();
    let page = Page {
        entry: entry(scan, root, position)?,
        previous: id_at(around.previous),
        next: id_at(around.next),
        series: members,
        after_reject: id_at(navigation::after_reject(count, series, view, position)),
    };
    Some((page, scan.entry_at(position)?.1.clone()))
}

fn entry(scan: &Scan, root: &Path, position: usize) -> Option<TimelineEntryDto> {
    let (id, photo) = scan.entry_at(position)?;
    Some(TimelineEntryDto::new(id, photo, root))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sito_core::timeline::build_timeline;

    use super::*;

    /// App state with a scan of the EXIF fixtures, and its id.
    fn scanned() -> (AppState, u64) {
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/exif");
        let state = AppState::default();
        let scan_id = state.set_timeline(build_timeline(&fixtures, |_| {}).unwrap());
        (state, scan_id)
    }

    fn request(scan_id: u64, id: usize, mode: ViewMode) -> ViewerRequest {
        ViewerRequest {
            id,
            scan_id,
            mode,
            gap: 10,
        }
    }

    /// Ids of the photos of the only series of the fixtures.
    fn series_ids(state: &AppState) -> Vec<usize> {
        let series = cached_series(state, Duration::from_secs(10)).unwrap();
        let scan = state.scan();
        let scan = scan.as_ref().unwrap();
        let s = series[0];
        (s.start..s.start + s.len)
            .map(|p| scan.entry_at(p).unwrap().0)
            .collect()
    }

    fn ids(entries: &[TimelineEntryDto]) -> Vec<usize> {
        entries
            .iter()
            .map(|e| serde_json::to_value(e).unwrap()["id"].as_u64().unwrap())
            .map(|id| usize::try_from(id).unwrap())
            .collect()
    }

    #[test]
    fn a_series_photo_shows_its_series_and_stays_in_it_after_a_reject() {
        let (state, scan_id) = scanned();
        let members = series_ids(&state);

        let shown = viewer_photo(&state, request(scan_id, members[0], ViewMode::Series)).unwrap();

        assert_eq!(ids(&shown.page.series), members);
        assert_eq!(shown.page.previous, None);
        assert_eq!(shown.page.next, Some(members[1]));
        assert_eq!(shown.page.after_reject, Some(members[1]));
    }

    #[test]
    fn a_single_photo_has_no_series_and_steps_through_all_photos() {
        let (state, scan_id) = scanned();
        let members = series_ids(&state);
        let single = state
            .scan()
            .as_ref()
            .unwrap()
            .photos()
            .map(|(id, _)| id)
            .find(|id| !members.contains(id))
            .unwrap();
        let position = state.scan().as_ref().unwrap().position_of(single).unwrap();

        let shown = viewer_photo(&state, request(scan_id, single, ViewMode::All)).unwrap();

        assert_eq!(shown.page.series, []);
        let scan = state.scan();
        let scan = scan.as_ref().unwrap();
        let id_at = |p: Option<usize>| p.and_then(|p| scan.entry_at(p)).map(|(id, _)| id);
        assert_eq!(shown.page.previous, id_at(position.checked_sub(1)));
        assert_eq!(shown.page.next, id_at(Some(position + 1)));
    }

    #[test]
    fn nothing_for_another_scan_or_a_photo_off_the_timeline() {
        let (state, scan_id) = scanned();
        let id = series_ids(&state)[0];

        assert_eq!(
            viewer_photo(&state, request(scan_id + 1, id, ViewMode::All)),
            None
        );
        state.scan().as_mut().unwrap().remove(id);
        assert_eq!(
            viewer_photo(&state, request(scan_id, id, ViewMode::All)),
            None
        );
    }

    #[test]
    fn details_give_the_file_size_and_the_upright_resolution() {
        let (state, scan_id) = scanned();
        let (id, path) = state
            .scan()
            .as_ref()
            .unwrap()
            .photos()
            .find(|(_, photo)| photo.path.ends_with("rotated_orientation-6.jpg"))
            .map(|(id, photo)| (id, photo.path.clone()))
            .unwrap();

        let shown = viewer_photo(&state, request(scan_id, id, ViewMode::All)).unwrap();

        // Stored 16 × 12, shown turned by 90°.
        assert_eq!(
            serde_json::to_value(shown.details).unwrap(),
            serde_json::json!({
                "sizeBytes": std::fs::metadata(path).unwrap().len(),
                "resolution": { "width": 12, "height": 16 },
            })
        );
        // The page's own fields stay at the top level.
        let value = serde_json::to_value(&shown).unwrap();
        assert!(value.get("entry").is_some() && value.get("details").is_some());
    }
}
