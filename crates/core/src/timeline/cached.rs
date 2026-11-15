//! Which found files the photo cache already knows: at their path, or
//! elsewhere when they were moved, renamed or copied outside Sito.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::Photo;
use crate::moved::{FileIdentity, MovedPhoto, match_moved, unique_positions};
use crate::photo_cache::{SharedCache, with_cache};
use crate::scan::{FileStamp, FoundFile, REJECTED_DIR, StampedFile};

/// The found files of a scan, split by what the cache knows of them.
#[derive(Debug, Default)]
pub(super) struct Split<'a> {
    /// Cached photos found at their cached path, unchanged.
    pub unchanged: Vec<Photo>,
    /// Cached photos found at another path (moved or copied), with their
    /// new paths. Not stored yet: only a complete scan stores them.
    pub relocated: Vec<Photo>,
    /// The moves among the relocated photos (copies are not moves).
    pub moves: Vec<MovedPhoto>,
    /// Files the cache does not know: they need reading.
    pub to_read: Vec<&'a FoundFile>,
}

/// Splits `files` into the photos up to date among the `known` cached ones
/// under the scanned folder, those the cache knows at another path (`known`
/// photos that are gone, then any photo in `cache`), and the files that
/// need reading.
pub(super) fn split_cached<'a>(
    files: &[&'a FoundFile],
    known: Vec<Photo>,
    cache: &SharedCache,
) -> Split<'a> {
    let found: HashSet<&PathBuf> = files.iter().map(|file| &file.path).collect();
    let (gone, known): (Vec<Photo>, Vec<Photo>) = known
        .into_iter()
        .partition(|photo| !found.contains(&photo.path));
    let known: HashMap<PathBuf, Photo> = known
        .into_iter()
        .map(|photo| (photo.path.clone(), photo))
        .collect();
    let mut split = Split::default();
    let mut unknown = Vec::new();
    for &file in files {
        match known
            .get(&file.path)
            .filter(|photo| is_unchanged(photo, file))
        {
            Some(photo) => split.unchanged.push(photo.clone()),
            None => unknown.push(file),
        }
    }
    let unknown = relocate_gone(&mut split, unknown, &gone);
    split.to_read = relocate_from_cache(&mut split, unknown, cache);
    split
}

/// Takes the `unknown` files that are one of the `gone` photos moved within
/// the scanned folder into `split`; returns the others.
fn relocate_gone<'a>(
    split: &mut Split<'a>,
    unknown: Vec<&'a FoundFile>,
    gone: &[Photo],
) -> Vec<&'a FoundFile> {
    let gone_identities: Vec<Option<FileIdentity>> = gone.iter().map(photo_identity).collect();
    let unknown_identities: Vec<Option<FileIdentity>> =
        unknown.iter().map(|file| file_identity(file)).collect();
    let mut taken = vec![false; unknown.len()];
    for (gone_index, file_index) in match_moved(&gone_identities, &unknown_identities) {
        let (Some(photo), Some(&found)) = (gone.get(gone_index), unknown.get(file_index)) else {
            continue;
        };
        if let Some(moved) = relocated(photo, found) {
            push_move(&mut split.moves, &photo.path, &found.path);
            split.relocated.push(moved);
            if let Some(taken) = taken.get_mut(file_index) {
                *taken = true;
            }
        }
    }
    unknown
        .into_iter()
        .zip(taken)
        .filter_map(|(file, taken)| (!taken).then_some(file))
        .collect()
}

/// Takes the `unknown` files that the cache knows at exactly one other path
/// into `split`: a move if that path is gone, a copy if it still exists.
/// Returns the others. Paths already moved from are not used again.
fn relocate_from_cache<'a>(
    split: &mut Split<'a>,
    unknown: Vec<&'a FoundFile>,
    cache: &SharedCache,
) -> Vec<&'a FoundFile> {
    let identities: Vec<Option<FileIdentity>> =
        unknown.iter().map(|file| file_identity(file)).collect();
    let unique = unique_positions(&identities);
    let moved_from: HashSet<PathBuf> = split.moves.iter().map(|m| m.from.clone()).collect();
    // One lock for all lookups; the file system is checked after it.
    let candidates: HashMap<usize, Photo> = with_cache(cache, |cache| {
        unique
            .iter()
            .filter_map(|(identity, position)| {
                let position = (*position)?;
                let file = unknown.get(position)?;
                let mut found = cache.by_identity(identity).ok()?;
                found.retain(|photo| photo.path != file.path && !moved_from.contains(&photo.path));
                let photo = found.pop().filter(|_| found.is_empty())?;
                Some((position, photo))
            })
            .collect()
    })
    .unwrap_or_default();
    let mut rest = Vec::new();
    for (position, file) in unknown.into_iter().enumerate() {
        let Some(photo) = candidates.get(&position) else {
            rest.push(file);
            continue;
        };
        let Some(moved) = relocated(photo, file) else {
            rest.push(file);
            continue;
        };
        if !photo.path.exists() {
            push_move(&mut split.moves, &photo.path, &file.path);
        }
        split.relocated.push(moved);
    }
    rest
}

/// Adds the move from `from` to `to` to `moves`, unless it goes into or
/// out of a rejected folder: Sito's own rejections and undos look like
/// that, and a rejected photo's mark stays at its original path.
fn push_move(moves: &mut Vec<MovedPhoto>, from: &Path, to: &Path) {
    if is_rejected(from) || is_rejected(to) {
        return;
    }
    moves.push(MovedPhoto {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
    });
}

/// Whether `path` is in a [`REJECTED_DIR`] folder.
fn is_rejected(path: &Path) -> bool {
    path.parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(REJECTED_DIR))
}

/// `photo` at the path of `file`, if its Takeout date file and RAW companion
/// match those of `file` by size and modification time (their paths change
/// along with the photo's).
fn relocated(photo: &Photo, file: &FoundFile) -> Option<Photo> {
    let same = stamp(photo.sidecar.as_ref()) == stamp(file.sidecar.as_ref())
        && stamp(photo.companion.as_ref()) == stamp(file.companion.as_ref());
    same.then(|| Photo {
        path: file.path.clone(),
        sidecar: file.sidecar.clone(),
        companion: file.companion.clone(),
        ..photo.clone()
    })
}

fn stamp(file: Option<&StampedFile>) -> Option<&FileStamp> {
    file.map(|file| &file.stamp)
}

/// Whether `file`, its Takeout date file and its RAW companion are as
/// cached in `photo`.
fn is_unchanged(photo: &Photo, file: &FoundFile) -> bool {
    photo.size == file.size
        && photo.modified == file.modified
        && photo.sidecar == file.sidecar
        && photo.companion == file.companion
}

fn photo_identity(photo: &Photo) -> Option<FileIdentity> {
    FileIdentity::new(&photo.path, photo.size, photo.modified)
}

fn file_identity(file: &FoundFile) -> Option<FileIdentity> {
    FileIdentity::new(&file.path, file.size, file.modified)
}
