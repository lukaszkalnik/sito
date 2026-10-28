//! Which Takeout date file belongs to which photo, by file name alone.

use std::collections::HashMap;

/// Suffix Google Takeout adds to a photo's name for its date file.
const SUPPLEMENTAL: &str = ".supplemental-metadata";
/// Extension of every Takeout date file.
const JSON: &str = ".json";
/// Takeout limits file names to this many characters (with `.json`) and cuts
/// longer date file names short; shorter names are never truncated.
const TRUNCATED_NAME_LEN: usize = 46;
/// Endings Google Photos gives the names of edited copies, in the export's
/// language (lower case). An edited copy has no date file of its own.
const EDITED_SUFFIXES: &[&str] = &[
    "-edited",
    "-edytowano",
    "-bearbeitet",
    "-modifié",
    "-editado",
    "-modificato",
    "-bewerkt",
    "-redigeret",
    "-redigert",
    "-redigerad",
    "-muokattu",
    "-upraveno",
    "-upravené",
    "-szerkesztett",
];

/// Pairs photos with their Takeout date files within one folder.
///
/// Returns `(photo index, sidecar index)` pairs. Every photo appears at most
/// once, and so does every sidecar, except that edited copies share their
/// original's. Names are compared ignoring letter case (Takeout writes e.g.
/// `10(6).jpg` next to `10.JPG.supplemental-metadata(6).json`). Rules, from
/// the most to the least specific:
///
/// 1. `IMG.jpg` ↔ `IMG.jpg.supplemental-metadata.json`
/// 2. duplicates: `IMG(1).jpg` ↔ `IMG.jpg.supplemental-metadata(1).json`
/// 3. older exports: `IMG.jpg` ↔ `IMG.jpg.json`, `IMG(1).jpg` ↔ `IMG.jpg(1).json`
/// 4. truncated long names: the date file's name (without `(n)` and `.json`)
///    is the start of `IMG.jpg.supplemental-metadata`; the longest one wins.
/// 5. edited copies: `IMG-edited.jpg` (in the export's language, e.g.
///    `IMG-edytowano.jpg`) shares the date file of `IMG.jpg`.
#[must_use]
pub fn match_sidecars(photos: &[&str], sidecars: &[&str]) -> Vec<(usize, usize)> {
    let photos: Vec<String> = photos.iter().map(|name| name.to_lowercase()).collect();
    let sidecars: Vec<String> = sidecars.iter().map(|name| name.to_lowercase()).collect();
    let index: HashMap<&str, usize> = sidecars
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();
    let mut taken = vec![false; sidecars.len()];
    let mut pairs = Vec::new();
    let mut unmatched = Vec::new();

    for (photo_index, photo) in photos.iter().enumerate() {
        match find_exact(photo, &index, &taken) {
            Some(sidecar) => {
                taken[sidecar] = true;
                pairs.push((photo_index, sidecar));
            }
            None => unmatched.push(photo_index),
        }
    }
    let sidecar_names: Vec<&str> = sidecars.iter().map(String::as_str).collect();
    let mut still_unmatched = Vec::new();
    for photo_index in unmatched {
        match truncated_match(&photos[photo_index], &sidecar_names, &taken) {
            Some(sidecar) => {
                taken[sidecar] = true;
                pairs.push((photo_index, sidecar));
            }
            None => still_unmatched.push(photo_index),
        }
    }
    let shared = vec![false; sidecars.len()];
    pairs.extend(still_unmatched.into_iter().filter_map(|photo_index| {
        let original = original_of(&photos[photo_index])?;
        find_exact(&original, &index, &shared).map(|sidecar| (photo_index, sidecar))
    }));
    pairs
}

/// Rules 1–3: the first date file `photo` accepts that is not `taken`.
fn find_exact(photo: &str, index: &HashMap<&str, usize>, taken: &[bool]) -> Option<usize> {
    exact_names(photo)
        .into_iter()
        .find_map(|name| index.get(name.as_str()).copied().filter(|&i| !taken[i]))
}

/// Whether `photo` is named like an edited copy (`IMG-edited.jpg`, in any of
/// the export languages).
#[must_use]
pub fn is_edited_copy(photo: &str) -> bool {
    original_of(&photo.to_lowercase()).is_some()
}

/// The original's name of an edited copy: `IMG-edited(1).jpg` → `IMG(1).jpg`.
fn original_of(photo: &str) -> Option<String> {
    let (stem, ext) = photo.rsplit_once('.')?;
    let (base, number) = split_number(stem).map_or((stem, None), |(b, n)| (b, Some(n)));
    let original = EDITED_SUFFIXES
        .iter()
        .find_map(|suffix| base.strip_suffix(suffix))
        .filter(|original| !original.is_empty())?;
    Some(match number {
        Some(n) => format!("{original}({n}).{ext}"),
        None => format!("{original}.{ext}"),
    })
}

/// The date file names rules 1–3 accept for `photo`, best first.
fn exact_names(photo: &str) -> Vec<String> {
    let mut names = vec![format!("{photo}{SUPPLEMENTAL}{JSON}")];
    if let Some((base, n)) = split_duplicate(photo) {
        names.push(format!("{base}{SUPPLEMENTAL}({n}){JSON}"));
        names.push(format!("{base}({n}){JSON}"));
    }
    names.push(format!("{photo}{JSON}"));
    names
}

/// Rule 4: the longest truncated date file name that starts the full one.
fn truncated_match(photo: &str, sidecars: &[&str], taken: &[bool]) -> Option<usize> {
    let (base, number) = match split_duplicate(photo) {
        Some((base, n)) => (base, Some(n)),
        None => (photo.to_owned(), None),
    };
    let full = format!("{base}{SUPPLEMENTAL}");
    let stem_len = base.rfind('.').unwrap_or(base.len());
    sidecars
        .iter()
        .enumerate()
        .filter(|(i, name)| !taken[*i] && name.chars().count() >= TRUNCATED_NAME_LEN)
        .filter_map(|(i, name)| {
            let (prefix, n) = split_sidecar(name)?;
            let fits = n == number && prefix.len() >= stem_len && full.starts_with(prefix);
            fits.then_some((i, prefix.len()))
        })
        .max_by_key(|&(_, len)| len)
        .map(|(i, _)| i)
}

/// `IMG(1).jpg` → `("IMG.jpg", "1")`; `None` without a `(n)` before the extension.
fn split_duplicate(photo: &str) -> Option<(String, &str)> {
    let (stem, ext) = photo.rsplit_once('.')?;
    let (base, number) = split_number(stem)?;
    Some((format!("{base}.{ext}"), number))
}

/// `name(1).json` → `("name", Some("1"))`, `name.json` → `("name", None)`.
fn split_sidecar(name: &str) -> Option<(&str, Option<&str>)> {
    let stem = name.strip_suffix(JSON)?;
    Some(match split_number(stem) {
        Some((base, n)) => (base, Some(n)),
        None => (stem, None),
    })
}

/// `name(12)` → `("name", "12")`.
fn split_number(text: &str) -> Option<(&str, &str)> {
    let inner = text.strip_suffix(')')?;
    let (base, number) = inner.rsplit_once('(')?;
    let valid =
        !base.is_empty() && !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit());
    valid.then_some((base, number))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: [(&str, &str); 0] = [];

    fn pairs<'a>(photos: &[&'a str], sidecars: &[&'a str]) -> Vec<(&'a str, &'a str)> {
        let mut found: Vec<_> = match_sidecars(photos, sidecars)
            .into_iter()
            .map(|(p, s)| (photos[p], sidecars[s]))
            .collect();
        found.sort_unstable();
        found
    }

    #[test]
    fn matches_the_supplemental_metadata_file() {
        assert_eq!(
            pairs(&["IMG_1.jpg"], &["IMG_1.jpg.supplemental-metadata.json"]),
            [("IMG_1.jpg", "IMG_1.jpg.supplemental-metadata.json")]
        );
    }

    #[test]
    fn matches_numbered_duplicates_to_their_own_file() {
        let photos = ["4.JPG", "4(1).JPG", "4(2).JPG"];
        let sidecars = [
            "4.JPG.supplemental-metadata(2).json",
            "4.JPG.supplemental-metadata.json",
            "4.JPG.supplemental-metadata(1).json",
        ];
        assert_eq!(
            pairs(&photos, &sidecars),
            [
                ("4(1).JPG", "4.JPG.supplemental-metadata(1).json"),
                ("4(2).JPG", "4.JPG.supplemental-metadata(2).json"),
                ("4.JPG", "4.JPG.supplemental-metadata.json"),
            ]
        );
    }

    #[test]
    fn a_photo_named_with_a_number_prefers_its_literal_file() {
        let sidecars = [
            "a(1).jpg.supplemental-metadata.json",
            "a.jpg.supplemental-metadata(1).json",
        ];
        assert_eq!(
            pairs(&["a(1).jpg"], &sidecars),
            [("a(1).jpg", "a(1).jpg.supplemental-metadata.json")]
        );
    }

    #[test]
    fn matches_older_json_names() {
        let photos = ["IMG.jpg", "IMG(1).jpg"];
        let sidecars = ["IMG.jpg(1).json", "IMG.jpg.json"];
        assert_eq!(
            pairs(&photos, &sidecars),
            [
                ("IMG(1).jpg", "IMG.jpg(1).json"),
                ("IMG.jpg", "IMG.jpg.json")
            ]
        );
    }

    #[test]
    fn matches_truncated_names_by_the_longest_prefix() {
        let photo = "Screenshot_20200101-120000_Some_Long_App.jpg";
        let sidecars = [
            "Screenshot_20200101-120000_Some_Long_App.j.json",
            "Screenshot_20200101-120000_Some_Long_App.jpg.s.json",
        ];
        assert_eq!(pairs(&[photo], &sidecars), [(photo, sidecars[1])]);
    }

    #[test]
    fn short_names_are_never_taken_as_truncated() {
        assert_eq!(
            pairs(&["IMG_1.jpg"], &["IMG_1.jpg.supp.json", "IMG_1.json"]),
            NONE
        );
    }

    #[test]
    fn a_truncated_file_must_reach_the_end_of_the_photo_stem() {
        let photo = "Screenshot_20200101-120000_Some_Long_App_Name.jpg";
        assert_eq!(
            pairs(
                &[photo],
                &["Screenshot_20200101-120000_Some_Long_App_Na.json"]
            ),
            NONE
        );
    }

    #[test]
    fn ignores_letter_case() {
        assert_eq!(
            pairs(&["10(6).jpg"], &["10.JPG.supplemental-metadata(6).json"]),
            [("10(6).jpg", "10.JPG.supplemental-metadata(6).json")]
        );
    }

    #[test]
    fn edited_copies_share_the_date_file_of_their_original() {
        let photos = ["415.jpg", "415-edytowano.jpg", "4-edited(1).jpg"];
        let sidecars = [
            "415.jpg.supplemental-metadata.json",
            "4.jpg.supplemental-metadata(1).json",
        ];
        assert_eq!(
            pairs(&photos, &sidecars),
            [
                ("4-edited(1).jpg", "4.jpg.supplemental-metadata(1).json"),
                ("415-edytowano.jpg", "415.jpg.supplemental-metadata.json"),
                ("415.jpg", "415.jpg.supplemental-metadata.json"),
            ]
        );
    }

    #[test]
    fn an_edited_copy_without_its_originals_file_stays_unmatched() {
        assert_eq!(
            pairs(&["-edited.jpg", "a-edited.jpg"], &["b.jpg.json"]),
            NONE
        );
    }

    #[test]
    fn each_sidecar_is_used_once() {
        let found = match_sidecars(&["a.jpg", "a.jpg"], &["a.jpg.supplemental-metadata.json"]);
        assert_eq!(found, [(0, 0)]);
    }

    #[test]
    fn ignores_album_metadata_and_unrelated_files() {
        assert_eq!(
            pairs(&["IMG.jpg"], &["metadane.json", "print-subscriptions.json"]),
            NONE
        );
    }
}
