//! Which photos look different after the pins changed, e.g. by undoing an
//! edit: the ones to point out to the user.

use std::collections::HashMap;
use std::time::Duration;

use super::pins::{Pin, Pins};
use super::{Series, group};
use crate::timeline::Photo;

/// Positions in `all` (the timeline with its rejected photos, sorted newest
/// first) of the photos whose placement differs between the series with
/// the pins `before` and with the pins `after`, both for `max_gap`; in
/// timeline order. Photos marked `true` in `removed` (same length as `all`)
/// are not shown and so never listed.
///
/// A photo changed when it went from a series to single or back, moved to
/// another series, or, while single, its pinned mark came or went. Each
/// series after is matched to the series before that shares the most photos
/// with it (one to one, larger overlaps first, ties to the newer series), so
/// a photo staying in its matched series did not move: undoing „Wyjmij z
/// serii” lists only the photo that came back, not the rest of its series.
/// Whether a series is edited by hand, or which manual id it has, is the
/// series' look, not its photos', and lists nothing.
#[must_use]
pub fn visibly_changed(
    all: &[Photo],
    removed: &[bool],
    max_gap: Duration,
    before: &Pins,
    after: &Pins,
) -> Vec<usize> {
    let old = series_index(&group(all, max_gap, before), all.len());
    let new = series_index(&group(all, max_gap, after), all.len());
    let matched = matching(&old, &new);
    let single_mark = |pins: &Pins, photo: &Photo| pins.get(&photo.path) == Some(Pin::Single);
    all.iter()
        .zip(removed)
        .enumerate()
        .filter(|(_, (_, gone))| !**gone)
        .filter(|&(position, (photo, _))| {
            match (
                old.get(position).copied().flatten(),
                new.get(position).copied().flatten(),
            ) {
                (None, None) => single_mark(before, photo) != single_mark(after, photo),
                (Some(_), None) | (None, Some(_)) => true,
                (Some(was), Some(is)) => matched.get(&is) != Some(&was),
            }
        })
        .map(|(position, _)| position)
        .collect()
}

/// For each of `len` positions, the index of the series it is in, if any.
fn series_index(series: &[Series], len: usize) -> Vec<Option<usize>> {
    let mut index = vec![None; len];
    for (number, s) in series.iter().enumerate() {
        for &member in &s.members {
            if let Some(slot) = index.get_mut(member) {
                *slot = Some(number);
            }
        }
    }
    index
}

/// Matches each new series (by index) to the old one sharing the most
/// photos with it, one to one, larger overlaps first; ties go to the newer
/// series (lower indices, as series are sorted by their newest photo).
fn matching(old: &[Option<usize>], new: &[Option<usize>]) -> HashMap<usize, usize> {
    let mut overlaps: HashMap<(usize, usize), usize> = HashMap::new();
    for (was, is) in old.iter().zip(new) {
        if let (Some(was), Some(is)) = (was, is) {
            *overlaps.entry((*is, *was)).or_default() += 1;
        }
    }
    let mut pairs: Vec<((usize, usize), usize)> = overlaps.into_iter().collect();
    pairs.sort_by_key(|&((is, was), count)| (std::cmp::Reverse(count), is, was));
    let mut matched: HashMap<usize, usize> = HashMap::new();
    let mut taken: Vec<usize> = Vec::new();
    for ((is, was), _) in pairs {
        if !matched.contains_key(&is) && !taken.contains(&was) {
            matched.insert(is, was);
            taken.push(was);
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::tests::shots;
    use super::super::{DEFAULT_MAX_GAP, ManualSeriesId};
    use super::*;

    const A: Option<&str> = Some("Canon EOS R6");
    const B: Option<&str> = Some("Pixel 8");

    /// Photos of one camera at the given seconds after noon, newest first.
    fn photos(seconds: &[i64]) -> Vec<Photo> {
        shots(&seconds.iter().map(|&s| (s, A)).collect::<Vec<_>>())
    }

    /// Pins for the photos taken at the given seconds (named `<s>.jpg`).
    fn pins(pinned: &[(i64, Pin)]) -> Pins {
        pinned
            .iter()
            .map(|&(s, pin)| (PathBuf::from(format!("{s}.jpg")), pin))
            .collect()
    }

    fn manual(id: i64) -> Pin {
        Pin::Series(ManualSeriesId(id))
    }

    /// The seconds of the changed photos, newest first.
    fn changed(photos: &[Photo], before: &Pins, after: &Pins) -> Vec<i64> {
        changed_without(photos, &vec![false; photos.len()], before, after)
    }

    fn changed_without(
        photos: &[Photo],
        removed: &[bool],
        before: &Pins,
        after: &Pins,
    ) -> Vec<i64> {
        visibly_changed(photos, removed, DEFAULT_MAX_GAP, before, after)
            .into_iter()
            .map(|position| {
                photos[position]
                    .path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| stem.parse().ok())
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn nothing_changed_lists_nothing() {
        let photos = photos(&[0, 2, 4, 30]);
        let pins = pins(&[(30, Pin::Single)]);

        assert_eq!(changed(&photos, &pins, &pins), Vec::<i64>::new());
        assert_eq!(
            changed(&[], &Pins::default(), &Pins::default()),
            Vec::<i64>::new()
        );
    }

    #[test]
    fn undoing_a_remove_lists_only_the_photo_that_came_back() {
        // Newest first: 4, 2, 0; 2 was taken out of the series.
        let photos = photos(&[0, 2, 4]);
        let taken_out = pins(&[(4, manual(1)), (2, Pin::Single), (0, manual(1))]);

        assert_eq!(changed(&photos, &taken_out, &Pins::default()), [2]);
    }

    #[test]
    fn undoing_a_join_lists_only_the_photo_that_went_back() {
        // Series 30–34 and 0–4; 30 was joined to the older series.
        let photos = photos(&[0, 2, 4, 30, 32, 34]);
        let joined = pins(&[
            (34, manual(1)),
            (32, manual(1)),
            (30, manual(2)),
            (4, manual(2)),
            (2, manual(2)),
            (0, manual(2)),
        ]);

        assert_eq!(changed(&photos, &joined, &Pins::default()), [30]);
    }

    #[test]
    fn undoing_a_cut_lists_the_smaller_part() {
        // Newest first: 6, 4, 2, 1, 0, cut before 2: 6–4 and the larger 2–0.
        let photos = photos(&[0, 1, 2, 4, 6]);
        let cut = pins(&[
            (6, manual(1)),
            (4, manual(1)),
            (2, manual(2)),
            (1, manual(2)),
            (0, manual(2)),
        ]);

        assert_eq!(changed(&photos, &cut, &Pins::default()), [6, 4]);
    }

    #[test]
    fn undoing_an_ungroup_lists_the_whole_series() {
        let photos = photos(&[0, 2, 4]);
        let ungrouped = pins(&[(4, Pin::Single), (2, Pin::Single), (0, Pin::Single)]);

        assert_eq!(changed(&photos, &ungrouped, &Pins::default()), [4, 2, 0]);
    }

    #[test]
    fn undoing_a_restore_of_a_cut_lists_the_smaller_part() {
        let photos = photos(&[0, 1, 2, 4, 6]);
        let cut = pins(&[
            (6, manual(1)),
            (4, manual(1)),
            (2, manual(2)),
            (1, manual(2)),
            (0, manual(2)),
        ]);

        assert_eq!(changed(&photos, &Pins::default(), &cut), [6, 4]);
    }

    #[test]
    fn a_pinned_mark_alone_is_listed() {
        // 30 is single either way; undoing its pin takes the mark away.
        let photos = photos(&[0, 30]);

        assert_eq!(
            changed(&photos, &pins(&[(30, Pin::Single)]), &Pins::default()),
            [30]
        );
        assert_eq!(
            changed(&photos, &Pins::default(), &pins(&[(0, Pin::Single)])),
            [0]
        );
    }

    #[test]
    fn a_series_edited_mark_alone_is_not_listed() {
        // A series made manual by a join and automatic again by its undo.
        let photos = photos(&[0, 2, 4]);
        let manual_series = pins(&[(4, manual(1)), (2, manual(1)), (0, manual(1))]);

        assert_eq!(
            changed(&photos, &manual_series, &Pins::default()),
            Vec::<i64>::new()
        );
    }

    #[test]
    fn a_changed_series_id_alone_is_not_listed() {
        let photos = photos(&[0, 2]);

        assert_eq!(
            changed(
                &photos,
                &pins(&[(2, manual(1)), (0, manual(1))]),
                &pins(&[(2, manual(5)), (0, manual(5))]),
            ),
            Vec::<i64>::new()
        );
    }

    #[test]
    fn a_photo_moved_to_another_series_is_listed() {
        // Two manual series of two photos each swap one photo.
        let photos = photos(&[0, 2, 4, 6]);
        let before = pins(&[
            (6, manual(1)),
            (4, manual(1)),
            (2, manual(2)),
            (0, manual(2)),
        ]);
        let after = pins(&[
            (6, manual(1)),
            (4, manual(1)),
            (2, manual(1)),
            (0, manual(2)),
        ]);

        assert_eq!(changed(&photos, &before, &after), [2]);
    }

    #[test]
    fn ties_go_to_the_newer_series() {
        // One series of four split into two equal halves: the newer half
        // keeps the match, the older half is listed.
        let photos = photos(&[0, 2, 4, 6]);
        let halves = pins(&[
            (6, manual(1)),
            (4, manual(1)),
            (2, manual(2)),
            (0, manual(2)),
        ]);

        assert_eq!(changed(&photos, &Pins::default(), &halves), [2, 0]);
    }

    #[test]
    fn rejected_photos_are_never_listed() {
        let photos = photos(&[0, 2, 4]);
        let ungrouped = pins(&[(4, Pin::Single), (2, Pin::Single), (0, Pin::Single)]);

        assert_eq!(
            changed_without(&photos, &[false, true, false], &ungrouped, &Pins::default()),
            [4, 0]
        );
    }

    #[test]
    fn series_of_other_cameras_are_left_alone() {
        // a: 0, 2 a series; b: 1, 3 a series, taken apart.
        let photos = shots(&[(0, A), (1, B), (2, A), (3, B)]);
        let ungrouped = pins(&[(3, Pin::Single), (1, Pin::Single)]);

        assert_eq!(changed(&photos, &ungrouped, &Pins::default()), [3, 1]);
    }
}
