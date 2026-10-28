//! Finding the Takeout date files of the photos in a folder.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::naming::{is_edited_copy, match_sidecars};
use super::sidecar::read_sidecar;
use crate::scan::is_photo;

/// Whether `path` may be a Takeout date file (a `.json` file, any case).
#[must_use]
pub fn is_sidecar_candidate(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
}

/// Pairs the photos of one folder with their Takeout date files.
///
/// Returns `(photo index, sidecar index)` pairs. Photos are matched by file
/// name first (see [`match_sidecars`]); photos still without a date file are
/// then matched by the `title` stored in the remaining date files, which are
/// read for that. Files whose name is not valid Unicode are never matched.
#[must_use]
pub fn pair_sidecars(photos: &[&Path], sidecars: &[&Path]) -> Vec<(usize, usize)> {
    let photo_names: Vec<&str> = photos.iter().map(|p| name_of(p)).collect();
    let sidecar_names: Vec<&str> = sidecars.iter().map(|p| name_of(p)).collect();
    let mut pairs = match_sidecars(&photo_names, &sidecar_names);
    if pairs.len() < photos.len() && pairs.len() < sidecars.len() {
        pairs.extend(pair_by_title(&photo_names, sidecars, &pairs));
    }
    pairs
}

/// The Takeout date file of the photo at `path`, if its folder has one.
#[must_use]
pub fn find_sidecar(path: &Path) -> Option<PathBuf> {
    find_in_folder(path).map(|found| found.path)
}

/// The Takeout date file that belongs to the photo at `path` alone: not one
/// it shares with other photos, as an edited copy shares its original's.
/// Moving the photo may take this file along.
#[must_use]
pub fn find_own_sidecar(path: &Path) -> Option<PathBuf> {
    let found = find_in_folder(path)?;
    let edited_copy = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_edited_copy);
    (found.users == 1 && !edited_copy).then_some(found.path)
}

/// A photo's date file and how many photos of the folder use it.
struct Found {
    path: PathBuf,
    users: usize,
}

fn find_in_folder(path: &Path) -> Option<Found> {
    let folder = path.parent()?;
    let (photos, sidecars): (Vec<PathBuf>, Vec<PathBuf>) = std::fs::read_dir(folder)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|file| is_photo(file) || is_sidecar_candidate(file))
        .partition(|file| is_photo(file));
    let photo_index = photos.iter().position(|photo| photo == path)?;
    let photos: Vec<&Path> = photos.iter().map(PathBuf::as_path).collect();
    let sidecar_paths: Vec<&Path> = sidecars.iter().map(PathBuf::as_path).collect();
    let pairs = pair_sidecars(&photos, &sidecar_paths);
    let (_, sidecar) = pairs.iter().find(|&&(photo, _)| photo == photo_index)?;
    Some(Found {
        path: sidecars[*sidecar].clone(),
        users: pairs.iter().filter(|(_, s)| s == sidecar).count(),
    })
}

/// Matches photos not yet in `pairs` to unused date files by their `title`.
fn pair_by_title(
    photo_names: &[&str],
    sidecars: &[&Path],
    pairs: &[(usize, usize)],
) -> Vec<(usize, usize)> {
    let used_photos: HashSet<usize> = pairs.iter().map(|&(photo, _)| photo).collect();
    let used_sidecars: HashSet<usize> = pairs.iter().map(|&(_, sidecar)| sidecar).collect();
    let mut by_title: HashMap<String, usize> = HashMap::new();
    for (index, path) in sidecars.iter().enumerate() {
        if used_sidecars.contains(&index) {
            continue;
        }
        // Files that are not date files (album metadata, other JSON) have no
        // capture time and are skipped here.
        if let Some(title) = read_sidecar(path).ok().and_then(|sidecar| sidecar.title) {
            by_title.entry(title).or_insert(index);
        }
    }
    photo_names
        .iter()
        .enumerate()
        .filter(|(index, name)| !used_photos.contains(index) && !name.is_empty())
        .filter_map(|(index, name)| by_title.remove(*name).map(|sidecar| (index, sidecar)))
        .collect()
}

fn name_of(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
}
