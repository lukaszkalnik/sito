//! Editing series by hand: the `edit_series` command, undoing an edit, and
//! keeping the open folder's pins in step with the action log.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sito_core::actions::ActionLog;
use sito_core::moved::MovedPhoto;
use sito_core::series::{Edit, EditAction, EditError, EditKind, Pins, plan_edit, visibly_changed};
use tauri::{AppHandle, Manager};

use crate::error::{CommandError, off_main_thread};
use crate::reject::{Actions, TIMELINE_CHANGED, announce, file_name, log_failed};
use crate::scan::state::{AppState, Scan};

/// An edit of the series, naming photos by their ids in the current scan.
/// Mirrors [`Edit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EditDto {
    /// Splits the photo's series before it.
    Cut {
        /// A photo of a series other than its first.
        photo: usize,
    },
    /// Takes the photo out of its series.
    Remove {
        /// A photo of a series.
        photo: usize,
    },
    /// Puts the photo into the series of `into`.
    Join {
        /// The photo to move.
        photo: usize,
        /// A photo of the series to join.
        into: usize,
    },
    /// Makes every photo of the series a single photo.
    Ungroup {
        /// A photo of the series.
        photo: usize,
    },
    /// Gives the photo's manual series, or the photo pinned on its own,
    /// back to automatic grouping.
    Restore {
        /// A photo of a manual series, or a pinned single photo.
        photo: usize,
    },
}

impl EditDto {
    /// The photo the edit is about (for a series, the one that names it).
    fn photo(self) -> usize {
        match self {
            Self::Cut { photo }
            | Self::Remove { photo }
            | Self::Join { photo, .. }
            | Self::Ungroup { photo }
            | Self::Restore { photo } => photo,
        }
    }
}

/// The outcome of an edit, for its toast.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditedDto {
    /// File name of the photo the edit was about.
    name: String,
    /// Ids of the photos that look different now (see [`visibly_changed`]),
    /// newest first.
    changed: Vec<usize>,
}

/// Applies `edit` to the series of the current scan as shown for
/// `max_gap_seconds`, stores it in the action log (so „Cofnij” can undo it)
/// and tells the windows that the timeline changed.
///
/// # Errors
///
/// [`CommandError::SeriesChanged`] if the edit does not fit the current
/// series (e.g. changed in another window), [`CommandError::Internal`] if
/// the action log cannot be used.
#[tauri::command]
pub async fn edit_series(
    edit: EditDto,
    max_gap_seconds: u32,
    app: AppHandle,
) -> Result<EditedDto, CommandError> {
    let max_gap = Duration::from_secs(max_gap_seconds.into());
    off_main_thread(move || {
        let actions = app.state::<Actions>().inner().lock();
        let state = app.state::<AppState>();
        let mut scan = state.scan();
        let scan = scan.as_mut().ok_or(CommandError::SeriesChanged)?;
        let edited = edit_in(scan, actions.log()?, edit, max_gap)?;
        announce(&app, TIMELINE_CHANGED);
        Ok(edited)
    })
    .await?
}

/// [`edit_series`] on the given scan and log.
pub(crate) fn edit_in(
    scan: &mut Scan,
    log: &ActionLog,
    edit: EditDto,
    max_gap: Duration,
) -> Result<EditedDto, CommandError> {
    let path = |id: usize| {
        scan.photo(id)
            .map(|photo| photo.path.clone())
            .ok_or(CommandError::SeriesChanged)
    };
    let name = file_name(&path(edit.photo())?);
    let edit = match edit {
        EditDto::Cut { photo } => Edit::Cut { at: path(photo)? },
        EditDto::Remove { photo } => Edit::Remove {
            photo: path(photo)?,
        },
        EditDto::Join { photo, into } => Edit::Join {
            photo: path(photo)?,
            into: path(into)?,
        },
        EditDto::Ungroup { photo } => Edit::Ungroup {
            series: path(photo)?,
        },
        EditDto::Restore { photo } => Edit::Restore {
            photo: path(photo)?,
        },
    };
    let (all, _) = scan.photos_with_removed();
    let next = log.next_series_id().map_err(|err| log_failed(&err))?;
    let changes =
        plan_edit(&all, max_gap, scan.pins(), &edit, next).map_err(|err| refused(&err))?;
    log.record_edit(edit.kind(), edit.photo(), &changes)
        .map_err(|err| log_failed(&err))?;
    let before = scan.pins().clone();
    scan.apply_pins(&changes);
    Ok(EditedDto {
        name,
        changed: changed_ids(scan, max_gap, &before),
    })
}

/// Which edit was undone. Mirrors [`EditKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EditKindDto {
    /// „Podziel serię przed tym zdjęciem”.
    Cut,
    /// „Wyjmij z serii”.
    Remove,
    /// „Dołącz do poprzedniej/następnej serii”.
    Join,
    /// „Rozdziel serię”.
    Ungroup,
    /// „Przywróć automatyczne grupowanie”.
    Restore,
}

impl From<EditKind> for EditKindDto {
    fn from(kind: EditKind) -> Self {
        match kind {
            EditKind::Cut => Self::Cut,
            EditKind::Remove => Self::Remove,
            EditKind::Join => Self::Join,
            EditKind::Ungroup => Self::Ungroup,
            EditKind::Restore => Self::Restore,
        }
    }
}

/// A series edit „Cofnij” took back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditUndoneDto {
    /// Which edit it was.
    pub(crate) edit: EditKindDto,
    /// File name of the photo the edit was about.
    pub(crate) name: String,
    /// Ids of the photos that look different now (see
    /// [`visibly_changed`]), newest first.
    pub(crate) changed: Vec<usize>,
}

/// Undoes the series edit `action` in the log and in `scan`, and works out
/// which photos look different now, with the series for `max_gap`.
///
/// # Errors
///
/// [`CommandError::EditUndoFailed`] if the action log cannot be used; then
/// nothing changes.
pub(crate) fn undo_edit(
    scan: &mut Scan,
    log: &ActionLog,
    action: &EditAction,
    max_gap: Duration,
) -> Result<EditUndoneDto, CommandError> {
    let before = scan.pins().clone();
    let changes = log.undo_edit(action.id).map_err(|err| {
        log::warn!("cannot undo a series edit: {err}");
        CommandError::EditUndoFailed
    })?;
    scan.apply_pins(&changes);
    Ok(EditUndoneDto {
        edit: action.kind.into(),
        name: file_name(&action.photo),
        changed: changed_ids(scan, max_gap, &before),
    })
}

/// Ids of the photos of `scan` that look different with its pins than with
/// the pins `before`, with the series for `max_gap`, newest first (see
/// [`visibly_changed`]).
fn changed_ids(scan: &Scan, max_gap: Duration, before: &Pins) -> Vec<usize> {
    let (all, removed) = scan.photos_with_removed();
    let ids: HashMap<&Path, usize> = scan
        .photos()
        .map(|(id, photo)| (photo.path.as_path(), id))
        .collect();
    visibly_changed(&all, &removed, max_gap, before, scan.pins())
        .into_iter()
        .filter_map(|position| ids.get(all.get(position)?.path.as_path()).copied())
        .collect()
}

fn refused(err: &EditError) -> CommandError {
    log::warn!("{err}");
    CommandError::SeriesChanged
}

/// Reads the pins of scan `scan_id`'s folder from the action log into the
/// scan. Without a log, or if the scan is no longer current, nothing
/// changes; a log that cannot be read leaves the folder without edits.
pub(crate) fn load_pins(app: &AppHandle, scan_id: u64) {
    let state = app.state::<AppState>();
    let Some(root) = state
        .scan()
        .as_ref()
        .filter(|scan| scan.id == scan_id)
        .map(|scan| scan.timeline.root.clone())
    else {
        return;
    };
    // Held until the pins are in the scan, so an edit cannot come between.
    let actions = app.state::<Actions>().inner().lock();
    let Ok(log) = actions.log() else {
        return;
    };
    match log.pins_under(&root) {
        Ok(pins) => {
            if let Some(scan) = state.scan().as_mut().filter(|scan| scan.id == scan_id) {
                scan.set_pins(pins);
            }
        }
        Err(err) => log::warn!("cannot read the series edits of {}: {err}", root.display()),
    }
}

/// Moves the pins of photos a scan found moved or renamed outside Sito to
/// their new paths in the action log.
pub(crate) fn follow_moves(app: &AppHandle, moves: &[MovedPhoto]) {
    if moves.is_empty() {
        return;
    }
    let actions = app.state::<Actions>().inner().lock();
    let Ok(log) = actions.log() else {
        return;
    };
    if let Err(err) = log.move_pins(moves) {
        log::warn!(
            "cannot move the series edits of {} moved photos: {err}",
            moves.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sito_core::series::{DEFAULT_MAX_GAP, Grouping, SeriesKind};
    use sito_core::timeline::build_timeline;

    use super::*;

    /// A scan of the EXIF fixtures; the two Pixel photos 5 s apart are the
    /// only series.
    fn scan() -> Scan {
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/exif");
        Scan::new(1, build_timeline(&fixtures, |_| {}).unwrap())
    }

    fn id_of(scan: &Scan, name: &str) -> usize {
        scan.photos()
            .find(|(_, photo)| photo.path.ends_with(name))
            .map(|(id, _)| id)
            .unwrap()
    }

    fn grouping(scan: &Scan) -> Grouping {
        let (all, removed) = scan.photos_with_removed();
        Grouping::new(&all, &removed, DEFAULT_MAX_GAP, scan.pins())
    }

    const NEWER: &str = "pixel_2024-06-01_12-00-05.jpg";
    const OLDER: &str = "pixel_2024-06-01_12-00-00.jpg";

    #[test]
    fn an_edit_changes_the_series_and_is_stored() {
        let mut scan = scan();
        let log = ActionLog::open_in_memory().unwrap();
        let before = scan.revision();
        let photo = id_of(&scan, OLDER);

        let edited = edit_in(&mut scan, &log, EditDto::Remove { photo }, DEFAULT_MAX_GAP);

        assert_eq!(edited.unwrap().name, OLDER);
        assert!(scan.revision() > before);
        let after = grouping(&scan);
        assert_eq!(after.series, []);
        assert_eq!(after.pinned_singles.len(), 2);
        // A fresh scan reading the log groups the same way.
        let mut fresh = self::scan();
        fresh.set_pins(log.pins_under(&fresh.timeline.root).unwrap());
        assert_eq!(grouping(&fresh), after);
    }

    #[test]
    fn the_grouping_sent_to_the_ui_lists_the_pinned_photos() {
        let mut scan = scan();
        let log = ActionLog::open_in_memory().unwrap();
        let photo = id_of(&scan, OLDER);
        edit_in(&mut scan, &log, EditDto::Remove { photo }, DEFAULT_MAX_GAP).unwrap();

        let dto = serde_json::to_value(crate::series::GroupingDto::from(&grouping(&scan))).unwrap();

        assert_eq!(dto["pinned"].as_array().map(Vec::len), Some(2), "{dto}");
    }

    #[test]
    fn restoring_gives_the_photos_back_to_automatic_grouping() {
        let mut scan = scan();
        let log = ActionLog::open_in_memory().unwrap();
        let photo = id_of(&scan, NEWER);
        edit_in(&mut scan, &log, EditDto::Ungroup { photo }, DEFAULT_MAX_GAP).unwrap();

        // Restoring one photo of the ungrouped pair restores both.
        edit_in(&mut scan, &log, EditDto::Restore { photo }, DEFAULT_MAX_GAP).unwrap();

        let series = grouping(&scan).series;
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].kind, SeriesKind::Automatic);
        assert!(log.pins_under(&scan.timeline.root).unwrap().is_empty());
    }

    /// File names of the photos `edited` lists as changed.
    fn changed(scan: &Scan, edited: &EditedDto) -> Vec<String> {
        edited
            .changed
            .iter()
            .map(|&id| file_name(&scan.photo(id).unwrap().path))
            .collect()
    }

    #[test]
    fn an_edit_lists_the_photos_that_look_different() {
        use crate::reject::tests::gallery;
        const COPY: &str = "pixel_copy.jpg";
        // A series of three: NEWER, OLDER and COPY (OLDER's time, after it by name).
        let dir = gallery();
        std::fs::copy(dir.path().join(OLDER), dir.path().join(COPY)).unwrap();
        let mut scan = Scan::new(1, build_timeline(dir.path(), |_| {}).unwrap());
        let log = ActionLog::open_in_memory().unwrap();
        let [newer, older] = [NEWER, OLDER].map(|name| id_of(&scan, name));
        let edit = |scan: &mut Scan, edit| {
            let edited = edit_in(scan, &log, edit, DEFAULT_MAX_GAP).unwrap();
            changed(scan, &edited)
        };

        // Taking out lists only the photo taken out, restoring brings back only it.
        assert_eq!(edit(&mut scan, EditDto::Remove { photo: older }), [OLDER]);
        assert_eq!(edit(&mut scan, EditDto::Restore { photo: older }), [OLDER]);
        // A cut before the second photo leaves the first one on its own.
        assert_eq!(edit(&mut scan, EditDto::Cut { photo: older }), [NEWER]);
        // Restoring from the single part gives back both parts in one series.
        assert_eq!(edit(&mut scan, EditDto::Restore { photo: newer }), [NEWER]);
        assert_eq!(grouping(&scan).series.len(), 1);
        // Restoring from the other part does too.
        edit(&mut scan, EditDto::Cut { photo: older });
        assert_eq!(edit(&mut scan, EditDto::Restore { photo: older }), [NEWER]);
        assert_eq!(
            edit(&mut scan, EditDto::Ungroup { photo: older }),
            [NEWER, OLDER, COPY]
        );
        assert_eq!(
            edit(&mut scan, EditDto::Restore { photo: newer }),
            [NEWER, OLDER, COPY]
        );
    }

    #[test]
    fn an_edit_that_no_longer_fits_is_refused() {
        let mut scan = scan();
        let log = ActionLog::open_in_memory().unwrap();
        let single = id_of(&scan, "no-exif.jpg");

        let refused = |scan: &mut Scan, edit| edit_in(scan, &log, edit, DEFAULT_MAX_GAP);

        assert_eq!(
            refused(&mut scan, EditDto::Remove { photo: single }),
            Err(CommandError::SeriesChanged)
        );
        assert_eq!(
            refused(&mut scan, EditDto::Remove { photo: 999 }),
            Err(CommandError::SeriesChanged)
        );
        assert!(log.pins_under(&scan.timeline.root).unwrap().is_empty());
    }

    #[test]
    fn the_edit_names_its_photo() {
        let photo = 3;
        assert_eq!(EditDto::Join { photo, into: 1 }.photo(), photo);
        assert_eq!(
            serde_json::from_value::<EditDto>(
                serde_json::json!({ "kind": "join", "photo": 3, "into": 1 })
            )
            .unwrap(),
            EditDto::Join { photo, into: 1 }
        );
    }
}
