//! Finding the Takeout date files of the photos in a folder.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::naming::match_sidecars;
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
    let folder = path.parent()?;
    let (photos, sidecars): (Vec<PathBuf>, Vec<PathBuf>) = std::fs::read_dir(folder)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|file| is_photo(file) || is_sidecar_candidate(file))
        .partition(|file| is_photo(file));
    let photo_index = photos.iter().position(|photo| photo == path)?;
    let photos: Vec<&Path> = photos.iter().map(PathBuf::as_path).collect();
    let sidecar_paths: Vec<&Path> = sidecars.iter().map(PathBuf::as_path).collect();
    pair_sidecars(&photos, &sidecar_paths)
        .into_iter()
        .find(|&(photo, _)| photo == photo_index)
        .map(|(_, sidecar)| sidecars[sidecar].clone())
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
