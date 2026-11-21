//! „Cofnij” over rejections and series edits, on real files in temp dirs.

use std::fs;
use std::time::Duration;

use serde_json::json;
use sito_core::series::{DEFAULT_MAX_GAP, Grouping};
use tempfile::TempDir;

use super::*;
use crate::reject::tests::{FOLDER, NEWER, OLDER, gallery, id_of, scan, started};
use crate::reject::{file_name, reject_in};
use crate::series::edit::{EditDto, EditKindDto, edit_in};

/// A copy of [`OLDER`] (same time, so in its series, after it by name).
const COPY: &str = "pixel_copy.jpg";
/// Two photos without a camera taken at the same time: a second series,
/// older than the Pixel one.
const DATED: &str = "datetime-only_2024-05-30_10-00-00.jpg";
const DATED_COPY: &str = "datetime-only_copy.jpg";

/// The fixtures with a series of three Pixel photos (`NEWER`, `OLDER`, `COPY`)
/// and one of two photos without a camera (`DATED`, `DATED_COPY`).
fn two_series() -> TempDir {
    let dir = gallery();
    fs::copy(dir.path().join(OLDER), dir.path().join(COPY)).unwrap();
    fs::copy(dir.path().join(DATED), dir.path().join(DATED_COPY)).unwrap();
    dir
}

/// File names of each series' photos, newest first, as the folder shows
/// them for `gap`, with the scan's pins.
fn series_at(scan: &Scan, gap: Duration) -> Vec<Vec<String>> {
    let (all, removed) = scan.photos_with_removed();
    let left: Vec<_> = all
        .iter()
        .zip(&removed)
        .filter(|(_, gone)| !**gone)
        .map(|(photo, _)| photo)
        .collect();
    Grouping::new(&all, &removed, gap, scan.pins())
        .series
        .iter()
        .map(|s| {
            s.members
                .iter()
                .map(|&m| file_name(&left[m].path))
                .collect()
        })
        .collect()
}

fn series(scan: &Scan) -> Vec<Vec<String>> {
    series_at(scan, DEFAULT_MAX_GAP)
}

fn names(list: &[&[&str]]) -> Vec<Vec<String>> {
    list.iter()
        .map(|s| s.iter().map(|&name| name.to_owned()).collect())
        .collect()
}

/// Makes `edit` with the default gap.
fn edit(scan: &mut Scan, actions: &ActionsState, edit: EditDto) {
    edit_in(scan, actions.log().unwrap(), edit, DEFAULT_MAX_GAP).unwrap();
}

/// Undoes the newest action, which must be a series edit.
fn undo_edit_in(scan: &mut Scan, actions: &ActionsState) -> EditUndoneDto {
    match undo_in(scan, actions, DEFAULT_MAX_GAP).unwrap() {
        Some(UndoneDto::Edit(undone)) => undone,
        other => panic!("not an undone edit: {other:?}"),
    }
}

/// Undoes the newest action, which must be a rejection.
fn undo_reject_in(scan: &mut Scan, actions: &ActionsState) -> RestoredDto {
    match undo_in(scan, actions, DEFAULT_MAX_GAP).unwrap() {
        Some(UndoneDto::Rejection(restored)) => restored,
        other => panic!("not an undone rejection: {other:?}"),
    }
}

fn changed_names(scan: &Scan, undone: &EditUndoneDto) -> Vec<String> {
    undone
        .changed
        .iter()
        .map(|&id| file_name(&scan.photo(id).unwrap().path))
        .collect()
}

const ORIGINAL: &[&[&str]] = &[&[NEWER, OLDER, COPY], &[DATED, DATED_COPY]];

#[test]
fn every_edit_undone_gives_back_the_series_before_it() {
    let dir = two_series();
    let mut scan = scan(dir.path());
    let actions = started(&dir.path().join("sito.db"));
    let older = id_of(&scan, OLDER);
    let dated = id_of(&scan, DATED);
    assert_eq!(series(&scan), names(ORIGINAL));
    let edits = [
        (EditDto::Cut { photo: older }, EditKindDto::Cut),
        (EditDto::Remove { photo: older }, EditKindDto::Remove),
        (
            EditDto::Join {
                photo: dated,
                into: older,
            },
            EditKindDto::Join,
        ),
        (EditDto::Ungroup { photo: older }, EditKindDto::Ungroup),
    ];

    for (made, kind) in edits {
        edit(&mut scan, &actions, made);
        assert_ne!(series(&scan), names(ORIGINAL), "{made:?}");

        let undone = undo_edit_in(&mut scan, &actions);

        assert_eq!(undone.edit, kind);
        assert_eq!(series(&scan), names(ORIGINAL), "{made:?}");
        let log = actions.log().unwrap();
        assert!(log.pins_under(dir.path()).unwrap().is_empty(), "{made:?}");
    }
    assert_eq!(undo_in(&mut scan, &actions, DEFAULT_MAX_GAP), Ok(None));
    assert_eq!(undo_state(&actions, dir.path()), UndoStateDto::Nothing);
}

#[test]
fn an_undone_edit_names_its_photo_and_the_photos_that_changed() {
    let dir = two_series();
    let mut scan = scan(dir.path());
    let actions = started(&dir.path().join("sito.db"));
    let older = id_of(&scan, OLDER);
    let dated = id_of(&scan, DATED);

    edit(&mut scan, &actions, EditDto::Remove { photo: older });
    let undone = undo_edit_in(&mut scan, &actions);
    assert_eq!(undone.name, OLDER);
    assert_eq!(changed_names(&scan, &undone), [OLDER]);

    edit(
        &mut scan,
        &actions,
        EditDto::Join {
            photo: dated,
            into: older,
        },
    );
    let undone = undo_edit_in(&mut scan, &actions);
    assert_eq!(undone.name, DATED);
    // The photo left behind was single meanwhile, so it changed too.
    assert_eq!(changed_names(&scan, &undone), [DATED, DATED_COPY]);

    edit(&mut scan, &actions, EditDto::Cut { photo: older });
    let undone = undo_edit_in(&mut scan, &actions);
    assert_eq!(changed_names(&scan, &undone), [NEWER]);

    edit(&mut scan, &actions, EditDto::Ungroup { photo: older });
    let undone = undo_edit_in(&mut scan, &actions);
    assert_eq!(changed_names(&scan, &undone), [NEWER, OLDER, COPY]);
}

#[test]
fn undoing_a_restore_brings_the_edited_stretch_back() {
    let dir = two_series();
    let mut scan = scan(dir.path());
    let actions = started(&dir.path().join("sito.db"));
    let older = id_of(&scan, OLDER);
    edit(&mut scan, &actions, EditDto::Cut { photo: older });
    let cut = series(&scan);
    // NEWER is single, pinned.
    assert_eq!(cut, names(&[&[OLDER, COPY], &[DATED, DATED_COPY]]));

    edit(&mut scan, &actions, EditDto::Restore { photo: older });
    assert_eq!(series(&scan), names(ORIGINAL));
    let undone = undo_edit_in(&mut scan, &actions);

    assert_eq!(undone.edit, EditKindDto::Restore);
    assert_eq!(series(&scan), cut);
    // Only the photo of the smaller part moved.
    assert_eq!(changed_names(&scan, &undone), [NEWER]);
    undo_edit_in(&mut scan, &actions);
    assert_eq!(series(&scan), names(ORIGINAL));
}

#[test]
fn sequences_of_edits_are_undone_newest_first() {
    let dir = two_series();
    let mut scan = scan(dir.path());
    let actions = started(&dir.path().join("sito.db"));
    let newer = id_of(&scan, NEWER);
    let older = id_of(&scan, OLDER);
    let dated = id_of(&scan, DATED);
    let dated_copy = id_of(&scan, DATED_COPY);

    // Join, then take the joined photo out again.
    edit(
        &mut scan,
        &actions,
        EditDto::Join {
            photo: dated,
            into: older,
        },
    );
    let joined = series(&scan);
    edit(&mut scan, &actions, EditDto::Remove { photo: dated });
    undo_edit_in(&mut scan, &actions);
    assert_eq!(series(&scan), joined);
    undo_edit_in(&mut scan, &actions);
    assert_eq!(series(&scan), names(ORIGINAL));

    // Cut, then join the part back.
    edit(&mut scan, &actions, EditDto::Cut { photo: older });
    let cut = series(&scan);
    edit(
        &mut scan,
        &actions,
        EditDto::Join {
            photo: newer,
            into: older,
        },
    );
    undo_edit_in(&mut scan, &actions);
    assert_eq!(series(&scan), cut);
    undo_edit_in(&mut scan, &actions);
    assert_eq!(series(&scan), names(ORIGINAL));

    // Ungroup, then restore; and edits of two series.
    edit(&mut scan, &actions, EditDto::Ungroup { photo: older });
    edit(&mut scan, &actions, EditDto::Restore { photo: older });
    edit(&mut scan, &actions, EditDto::Remove { photo: dated_copy });
    let after_three = series(&scan);
    undo_edit_in(&mut scan, &actions);
    assert_eq!(series(&scan), names(ORIGINAL));
    assert_ne!(series(&scan), after_three);
    undo_edit_in(&mut scan, &actions);
    assert_eq!(series(&scan), names(&[&[DATED, DATED_COPY]]));
    undo_edit_in(&mut scan, &actions);
    assert_eq!(series(&scan), names(ORIGINAL));
}

#[test]
fn edits_and_rejections_share_one_history() {
    let dir = two_series();
    let mut scan = scan(dir.path());
    let mut actions = started(&dir.path().join("sito.db"));
    let older = id_of(&scan, OLDER);
    let copy = id_of(&scan, COPY);

    // An edit, then a rejection of a photo of that series.
    edit(&mut scan, &actions, EditDto::Remove { photo: older });
    reject_in(&mut scan, &mut actions, copy, FOLDER).unwrap();
    assert_eq!(undo_reject_in(&mut scan, &actions).name, COPY);
    assert_eq!(undo_edit_in(&mut scan, &actions).name, OLDER);
    assert_eq!(series(&scan), names(ORIGINAL));

    // A rejection, then an edit of its series.
    reject_in(&mut scan, &mut actions, copy, FOLDER).unwrap();
    edit(&mut scan, &actions, EditDto::Ungroup { photo: older });
    assert_eq!(undo_edit_in(&mut scan, &actions).edit, EditKindDto::Ungroup);
    assert_eq!(undo_reject_in(&mut scan, &actions).name, COPY);
    assert_eq!(series(&scan), names(ORIGINAL));
    assert!(dir.path().join(COPY).exists());
    assert_eq!(undo_in(&mut scan, &actions, DEFAULT_MAX_GAP), Ok(None));
}

#[test]
fn an_edit_of_a_series_with_a_rejected_photo_undoes_around_it() {
    let dir = two_series();
    let mut scan = scan(dir.path());
    let mut actions = started(&dir.path().join("sito.db"));
    let older = id_of(&scan, OLDER);
    let copy = id_of(&scan, COPY);
    reject_in(&mut scan, &mut actions, copy, FOLDER).unwrap();

    edit(&mut scan, &actions, EditDto::Ungroup { photo: older });
    let undone = undo_edit_in(&mut scan, &actions);

    // The rejected photo is not shown, so not reported.
    assert_eq!(changed_names(&scan, &undone), [NEWER, OLDER]);
    assert_eq!(
        series(&scan),
        names(&[&[NEWER, OLDER], &[DATED, DATED_COPY]])
    );
}

#[test]
fn an_edit_is_undone_at_the_current_gap() {
    let dir = two_series();
    let mut scan = scan(dir.path());
    let actions = started(&dir.path().join("sito.db"));
    let short = Duration::from_secs(1);
    // NEWER is 5 s before the other two Pixel photos.
    let at_short_gap = series_at(&scan, short);
    assert_eq!(at_short_gap, names(&[&[OLDER, COPY], &[DATED, DATED_COPY]]));
    let copy = id_of(&scan, COPY);

    edit(&mut scan, &actions, EditDto::Remove { photo: copy });
    let undone = undo_in(&mut scan, &actions, short).unwrap();

    assert!(matches!(undone, Some(UndoneDto::Edit(_))));
    assert_eq!(series_at(&scan, short), at_short_gap);
}

#[test]
fn an_edit_is_undone_after_a_restart() {
    let dir = two_series();
    let db = dir.path().join("sito.db");
    {
        let mut scan = scan(dir.path());
        let actions = started(&db);
        let older = id_of(&scan, OLDER);
        edit(&mut scan, &actions, EditDto::Cut { photo: older });
    }

    let mut scan = scan(dir.path());
    let actions = started(&db);
    scan.set_pins(actions.log().unwrap().pins_under(dir.path()).unwrap());
    assert_eq!(undo_state(&actions, dir.path()), UndoStateDto::Available);
    assert_ne!(series(&scan), names(ORIGINAL));

    assert_eq!(undo_edit_in(&mut scan, &actions).edit, EditKindDto::Cut);
    assert_eq!(series(&scan), names(ORIGINAL));
}

#[test]
fn an_edit_undoes_a_photo_restored_under_a_new_name() {
    let dir = two_series();
    let mut scan = scan(dir.path());
    let mut actions = started(&dir.path().join("sito.db"));
    let copy = id_of(&scan, COPY);
    edit(&mut scan, &actions, EditDto::Remove { photo: copy });
    reject_in(&mut scan, &mut actions, copy, FOLDER).unwrap();
    fs::write(dir.path().join(COPY), "another file").unwrap();
    let restored = undo_reject_in(&mut scan, &actions);
    let renamed = restored.restored_name.unwrap();
    assert_eq!(
        series(&scan),
        names(&[&[NEWER, OLDER], &[DATED, DATED_COPY]])
    );

    let undone = undo_edit_in(&mut scan, &actions);

    assert_eq!(changed_names(&scan, &undone), [renamed.as_str()]);
    assert_eq!(
        series(&scan),
        names(&[&[NEWER, OLDER, &renamed], &[DATED, DATED_COPY]])
    );
    assert!(
        actions
            .log()
            .unwrap()
            .pins_under(dir.path())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn an_edit_of_a_parent_folder_is_not_offered_in_another_folder() {
    let dir = two_series();
    let other = TempDir::new().unwrap();
    let mut scan = scan(dir.path());
    let actions = started(&dir.path().join("sito.db"));
    let older = id_of(&scan, OLDER);
    edit(&mut scan, &actions, EditDto::Remove { photo: older });

    assert_eq!(undo_state(&actions, other.path()), UndoStateDto::Nothing);
    assert_eq!(undo_state(&actions, dir.path()), UndoStateDto::Available);
    // The root of the drive (`/`, or `D:\` on Windows) holds the folder too.
    let root = dir.path().ancestors().last().unwrap();
    assert_eq!(undo_state(&actions, root), UndoStateDto::Available);
}

#[test]
fn outcomes_and_states_serialize_as_camel_case() {
    assert_eq!(
        serde_json::to_value(UndoneDto::Rejection(RestoredDto {
            id: Some(3),
            name: "IMG_1.jpg".into(),
            restored_name: None,
        }))
        .unwrap(),
        json!({ "kind": "rejection", "id": 3, "name": "IMG_1.jpg", "restoredName": null })
    );
    let kinds = [
        (EditKindDto::Cut, "cut"),
        (EditKindDto::Remove, "remove"),
        (EditKindDto::Join, "join"),
        (EditKindDto::Ungroup, "ungroup"),
        (EditKindDto::Restore, "restore"),
    ];
    for (edit, text) in kinds {
        assert_eq!(
            serde_json::to_value(UndoneDto::Edit(EditUndoneDto {
                edit,
                name: "IMG_1.jpg".into(),
                changed: vec![4, 2],
            }))
            .unwrap(),
            json!({ "kind": "edit", "edit": text, "name": "IMG_1.jpg", "changed": [4, 2] })
        );
    }
    let states = [
        (UndoStateDto::Available, "available"),
        (UndoStateDto::Nothing, "none"),
        (UndoStateDto::TrashOnly, "trashOnly"),
    ];
    for (state, kind) in states {
        assert_eq!(
            serde_json::to_value(state).unwrap(),
            json!({ "kind": kind })
        );
    }
    assert_eq!(
        serde_json::to_value(CommandError::EditUndoFailed).unwrap(),
        json!({ "code": "editUndoFailed" })
    );
}
