use std::cell::Cell;

use tempfile::TempDir;

use super::*;
use crate::cache_dir::{photo_cache_file, remove_cache};

/// The relative path of the `n`th fake thumbnail.
fn thumb(n: u8) -> PathBuf {
    let hex = format!("{n:02x}").repeat(32);
    Path::new(THUMBNAILS_DIR)
        .join(&hex[..2])
        .join(format!("{hex}.jpg"))
}

/// Writes a cache of `count` thumbnails and a photo cache file into `dir`.
fn write_cache(dir: &Path, count: u8) {
    for n in 0..count {
        let path = dir.join(thumb(n));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("thumbnail {n}")).unwrap();
    }
    fs::write(photo_cache_file(dir), b"photo data").unwrap();
}

/// The files under `dir`, relative to it, sorted.
fn files_in(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(dir)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().strip_prefix(dir).unwrap().to_path_buf())
        .collect();
    files.sort();
    files
}

/// The files of a cache of `count` thumbnails, sorted.
fn cache_files(count: u8) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = (0..count).map(thumb).collect();
    files.push(PathBuf::from(PHOTO_CACHE_FILE));
    files.sort();
    files
}

fn with_no_media(mut files: Vec<PathBuf>) -> Vec<PathBuf> {
    files.push(PathBuf::from(NO_MEDIA_FILE));
    files.sort();
    files
}

/// Copies the cache as [`move_cache`] does across drives, long paths included.
fn move_cache_copying(from: &Path, to: &Path) {
    let long = |path: &Path| crate::long_path::verbatim(path).unwrap();
    move_with(
        &long(from),
        &long(to),
        Strategy::Copy,
        plain_copy,
        &|| false,
        |_| {},
    )
    .unwrap();
}

fn copy_all(from: &Path, to: &Path) -> Result<MoveOutcome, MoveError> {
    move_with(from, to, Strategy::Copy, plain_copy, &|| false, |_| {})
}

#[test]
fn on_one_drive_the_cache_is_renamed() {
    let root = TempDir::new().unwrap();
    let (from, to) = (root.path().join("old"), root.path().join("new/Sito-cache"));
    write_cache(&from, 3);
    fs::write(from.join("EBWebView"), b"not the cache").unwrap();

    let outcome = move_cache(&from, &to, || false, |_| {}).unwrap();

    assert_eq!(outcome, MoveOutcome::Renamed);
    assert_eq!(files_in(&to), with_no_media(cache_files(3)));
    assert_eq!(files_in(&from), [PathBuf::from("EBWebView")]);
}

#[test]
fn a_copy_holds_every_file_unchanged_and_leaves_the_old_one() {
    let root = TempDir::new().unwrap();
    let (from, to) = (root.path().join("old"), root.path().join("new/Sito-cache"));
    write_cache(&from, 3);

    assert_eq!(copy_all(&from, &to).unwrap(), MoveOutcome::Copied);

    assert_eq!(files_in(&to), with_no_media(cache_files(3)));
    for file in cache_files(3) {
        assert_eq!(
            fs::read(to.join(&file)).unwrap(),
            fs::read(from.join(&file)).unwrap()
        );
    }
    assert_eq!(files_in(&from), cache_files(3));
}

#[test]
fn removing_the_old_copy_leaves_only_what_was_not_the_cache() {
    let root = TempDir::new().unwrap();
    let from = root.path().join("old");
    write_cache(&from, 2);
    fs::write(from.join("EBWebView"), b"not the cache").unwrap();
    copy_all(&from, &root.path().join("new")).unwrap();

    remove_cache(&from).unwrap();

    assert_eq!(files_in(&from), [PathBuf::from("EBWebView")]);
    assert!(!from.join(THUMBNAILS_DIR).exists());
}

#[test]
fn removing_a_chosen_cache_folder_removes_the_folder_too() {
    let root = TempDir::new().unwrap();
    let (from, to) = (root.path().join("d/Sito-cache"), root.path().join("new"));
    write_cache(&from, 2);
    fs::write(from.join(NO_MEDIA_FILE), b"").unwrap();
    copy_all(&from, &to).unwrap();

    remove_cache(&from).unwrap();

    assert!(!from.exists());
    assert!(root.path().join("d").exists());
}

#[test]
fn progress_counts_every_file() {
    let root = TempDir::new().unwrap();
    let from = root.path().join("old");
    write_cache(&from, 4);
    let mut seen = Vec::new();

    move_with(
        &from,
        &root.path().join("new"),
        Strategy::Copy,
        plain_copy,
        &|| false,
        |progress| seen.push((progress.done, progress.total)),
    )
    .unwrap();

    assert_eq!(seen, [(1, 5), (2, 5), (3, 5), (4, 5), (5, 5)]);
}

/// A copy that fails for the third file.
fn failing_copy(from: &Path, to: &Path) -> io::Result<u64> {
    thread_local!(static CALLS: Cell<u32> = const { Cell::new(0) });
    let call = CALLS.with(|calls| {
        calls.set(calls.get() + 1);
        calls.get()
    });
    if call == 3 {
        return Err(io::Error::other("disk full"));
    }
    fs::copy(from, to)
}

/// A copy whose result differs from the original.
fn damaging_copy(_from: &Path, to: &Path) -> io::Result<u64> {
    fs::write(to, b"damaged")?;
    Ok(7)
}

#[test]
fn a_failing_copy_leaves_the_old_cache_and_no_partial_copy() {
    let root = TempDir::new().unwrap();
    let (from, to) = (root.path().join("old"), root.path().join("new/Sito-cache"));
    write_cache(&from, 4);

    let result = move_with(&from, &to, Strategy::Copy, failing_copy, &|| false, |_| {});

    assert!(matches!(result, Err(MoveError::Io { .. })), "{result:?}");
    assert_eq!(files_in(&from), cache_files(4));
    assert!(!to.exists());
}

#[test]
fn a_copy_that_differs_is_refused_and_removed() {
    let root = TempDir::new().unwrap();
    let (from, to) = (root.path().join("old"), root.path().join("new"));
    write_cache(&from, 2);

    let result = move_with(&from, &to, Strategy::Copy, damaging_copy, &|| false, |_| {});

    assert!(matches!(result, Err(MoveError::Mismatch(_))), "{result:?}");
    assert_eq!(files_in(&from), cache_files(2));
    assert!(!to.exists());
}

#[test]
fn a_stopped_copy_leaves_the_old_cache_and_no_partial_copy() {
    let root = TempDir::new().unwrap();
    let (from, to) = (root.path().join("old"), root.path().join("new"));
    write_cache(&from, 4);
    let copied = Cell::new(0);

    let result = move_with(
        &from,
        &to,
        Strategy::Copy,
        plain_copy,
        &|| copied.get() >= 2,
        |progress| copied.set(progress.done),
    );

    assert!(matches!(result, Err(MoveError::Stopped)), "{result:?}");
    assert_eq!(files_in(&from), cache_files(4));
    assert!(!to.exists());
}

#[test]
fn a_target_folder_that_was_there_stays_after_a_failed_copy() {
    let root = TempDir::new().unwrap();
    let (from, to) = (root.path().join("old"), root.path().join("new"));
    write_cache(&from, 2);
    fs::create_dir_all(&to).unwrap();
    fs::write(to.join("notes.txt"), b"mine").unwrap();

    move_with(&from, &to, Strategy::Copy, damaging_copy, &|| false, |_| {}).unwrap_err();

    assert_eq!(files_in(&to), [PathBuf::from("notes.txt")]);
}

#[test]
fn an_empty_cache_moves_as_well() {
    let root = TempDir::new().unwrap();
    let (from, to) = (root.path().join("old"), root.path().join("new"));
    fs::create_dir_all(&from).unwrap();

    assert_eq!(
        move_cache(&from, &to, || false, |_| {}).unwrap(),
        MoveOutcome::Renamed
    );
    assert_eq!(copy_all(&from, &to).unwrap(), MoveOutcome::Copied);
    assert_eq!(files_in(&to), Vec::<PathBuf>::new());
    let chosen = root.path().join("Sito-cache");
    copy_all(&from, &chosen).unwrap();
    assert_eq!(files_in(&chosen), [PathBuf::from(NO_MEDIA_FILE)]);
}

#[test]
fn non_ascii_and_long_paths_move() {
    let root = TempDir::new().unwrap();
    let from = root.path().join("Pamięć podręczna – ąćęłńóśźż");
    let deep = (0..6).fold(root.path().join("Zdjęcia"), |path, n| {
        path.join(format!("{n}-{}", "długa nazwa folderu ".repeat(3)))
    });
    let to = deep.join("Sito-cache");
    assert!(to.as_os_str().len() > 300);
    write_cache(&from, 2);

    move_cache_copying(&from, &to);
    // Listed in the long form, as the move works with it.
    let long = crate::long_path::verbatim(&to).unwrap();
    assert_eq!(files_in(&long), with_no_media(cache_files(2)));
    remove_cache(&from).unwrap();

    let back = root.path().join("z powrotem – żółw");
    assert_eq!(
        move_cache(&to, &back, || false, |_| {}).unwrap(),
        MoveOutcome::Renamed
    );
    assert_eq!(files_in(&back), cache_files(2));
}

#[test]
fn a_cache_left_in_the_target_is_merged_into() {
    let root = TempDir::new().unwrap();
    let (from, to) = (root.path().join("old"), root.path().join("new"));
    write_cache(&from, 2);
    // A cache used there before: one thumbnail of the same name, one other.
    write_cache(&to, 3);
    fs::write(to.join(thumb(0)), b"older thumbnail").unwrap();
    fs::write(photo_cache_file(&to), b"older photo data").unwrap();

    // The thumbnail folder there cannot be renamed onto, so it is copied.
    assert_eq!(
        move_cache(&from, &to, || false, |_| {}).unwrap(),
        MoveOutcome::Copied
    );

    assert_eq!(files_in(&to), cache_files(3));
    assert_eq!(fs::read(to.join(thumb(0))).unwrap(), b"thumbnail 0");
    assert_eq!(fs::read(to.join(thumb(2))).unwrap(), b"thumbnail 2");
    assert_eq!(fs::read(photo_cache_file(&to)).unwrap(), b"photo data");
}

#[test]
fn a_target_that_cannot_be_created_is_not_writable() {
    let root = TempDir::new().unwrap();
    let from = root.path().join("old");
    write_cache(&from, 1);
    let blocked = root.path().join("file");
    fs::write(&blocked, b"a file, not a folder").unwrap();

    let result = move_cache(&from, &blocked.join("Sito-cache"), || false, |_| {});

    assert!(
        matches!(result, Err(MoveError::NotWritable { .. })),
        "{result:?}"
    );
    assert_eq!(files_in(&from), cache_files(1));
}

#[cfg(target_os = "linux")]
#[test]
fn across_file_systems_the_cache_is_copied() {
    use std::os::unix::fs::MetadataExt;

    let root = TempDir::new().unwrap();
    let Ok(other) = TempDir::new_in("/dev/shm") else {
        return;
    };
    let device = |path: &Path| fs::metadata(path).unwrap().dev();
    if device(root.path()) == device(other.path()) {
        // No second file system here; the forced copy covers the same path.
        return;
    }
    let (from, to) = (root.path().join("old"), other.path().join("Sito-cache"));
    write_cache(&from, 3);

    assert_eq!(
        move_cache(&from, &to, || false, |_| {}).unwrap(),
        MoveOutcome::Copied
    );
    assert_eq!(files_in(&to), with_no_media(cache_files(3)));
    assert_eq!(files_in(&from), cache_files(3));
}
