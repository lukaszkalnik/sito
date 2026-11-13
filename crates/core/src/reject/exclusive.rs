//! Moving a file without ever replacing another one: the move and the check
//! that the destination is free are one system call, so a file that another
//! program puts there in between is never overwritten.

#![expect(
    unsafe_code,
    reason = "one system call per platform; std's rename replaces an existing destination"
)]

use std::io;
use std::path::Path;

/// Moves the file `from` to `to` on the same volume.
///
/// # Errors
///
/// [`io::ErrorKind::AlreadyExists`] if `to` exists (it is left untouched), or
/// the system's error if the file cannot be moved.
#[cfg(windows)]
pub(super) fn move_exclusive(from: &Path, to: &Path) -> io::Result<()> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    }

    let from = wide::verbatim(from)?;
    let to = wide::verbatim(to)?;
    // No flags: without `MOVEFILE_REPLACE_EXISTING` an existing destination
    // is refused, without `MOVEFILE_COPY_ALLOWED` the move stays on one volume.
    // SAFETY: both are NUL-terminated UTF-16 strings that outlive the call,
    // which only reads them.
    let moved = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) };
    if moved == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Moves the file `from` to `to` on the same volume.
///
/// On a volume that cannot do this in one step (exFAT and FAT cards, some
/// network shares), the destination is checked first and the file then
/// renamed, which leaves a short gap between the two.
///
/// # Errors
///
/// [`io::ErrorKind::AlreadyExists`] if `to` exists (it is left untouched), or
/// the system's error if the file cannot be moved.
#[cfg(unix)]
pub(super) fn move_exclusive(from: &Path, to: &Path) -> io::Result<()> {
    match rename_exclusive(from, to) {
        Err(err) if is_unsupported(&err) => rename_after_check(from, to),
        result => result,
    }
}

/// The exclusive rename of macOS.
#[cfg(target_os = "macos")]
fn rename_exclusive(from: &Path, to: &Path) -> io::Result<()> {
    let from = c_path(from)?;
    let to = c_path(to)?;
    // SAFETY: both are NUL-terminated strings that outlive the call, which
    // only reads them.
    let result = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// The exclusive rename of Linux.
#[cfg(target_os = "linux")]
fn rename_exclusive(from: &Path, to: &Path) -> io::Result<()> {
    let from = c_path(from)?;
    let to = c_path(to)?;
    // SAFETY: both are NUL-terminated strings that outlive the call, which
    // only reads them.
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Other Unix systems have no exclusive rename Sito knows of.
#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn rename_exclusive(_from: &Path, _to: &Path) -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

/// Whether `err` says that the volume or the system has no exclusive rename
/// (as opposed to refusing this move).
#[cfg(unix)]
fn is_unsupported(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Unsupported
        || matches!(
            err.raw_os_error(),
            Some(libc::ENOTSUP | libc::EINVAL | libc::ENOSYS)
        )
}

/// The move for volumes without an exclusive rename: check, then rename.
#[cfg(unix)]
fn rename_after_check(from: &Path, to: &Path) -> io::Result<()> {
    if to.symlink_metadata().is_ok() {
        return Err(io::Error::from(io::ErrorKind::AlreadyExists));
    }
    std::fs::rename(from, to)
}

/// `path` as a C string, with its bytes unchanged.
#[cfg(unix)]
fn c_path(path: &Path) -> io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;

    std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))
}

#[cfg(windows)]
mod wide {
    //! Paths as the wide strings the Windows calls take.

    use std::ffi::OsString;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Component, Path, Prefix};

    /// `path` as a NUL-terminated UTF-16 string in its verbatim form
    /// (`\\?\C:\...`, `\\?\UNC\server\share\...`), which is not limited to
    /// 260 characters.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is empty or contains a NUL.
    pub(super) fn verbatim(path: &Path) -> io::Result<Vec<u16>> {
        // Verbatim paths are taken literally, so make it absolute and
        // resolve `.` and `..` first.
        let absolute = std::path::absolute(path)?;
        let mut components = absolute.components();
        let mut text = OsString::new();
        match components.next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(_) => {
                    text.push(r"\\?\");
                    text.push(prefix.as_os_str());
                }
                Prefix::UNC(server, share) => {
                    text.push(r"\\?\UNC\");
                    text.push(server);
                    text.push(r"\");
                    text.push(share);
                }
                Prefix::Verbatim(_)
                | Prefix::VerbatimUNC(..)
                | Prefix::VerbatimDisk(_)
                | Prefix::DeviceNS(_) => text.push(prefix.as_os_str()),
            },
            Some(_) | None => text.push(absolute.as_os_str()),
        }
        text.push(components.as_path().as_os_str());
        let wide: Vec<u16> = text.encode_wide().collect();
        if wide.contains(&0) {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        Ok(wide.into_iter().chain([0]).collect())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn text(path: &str) -> String {
            let wide = verbatim(Path::new(path)).unwrap();
            String::from_utf16(&wide[..wide.len() - 1]).unwrap()
        }

        #[test]
        fn a_drive_path_becomes_verbatim() {
            assert_eq!(
                text(r"C:\Zdjęcia\.\Wakacje\..\IMG_1.jpg"),
                r"\\?\C:\Zdjęcia\IMG_1.jpg"
            );
        }

        #[test]
        fn a_network_path_becomes_verbatim_unc() {
            assert_eq!(
                text(r"\\server\share\Zdjęcia\IMG_1.jpg"),
                r"\\?\UNC\server\share\Zdjęcia\IMG_1.jpg"
            );
        }

        #[test]
        fn a_verbatim_path_stays_as_it_is() {
            assert_eq!(
                text(r"\\?\C:\Zdjęcia\IMG_1.jpg"),
                r"\\?\C:\Zdjęcia\IMG_1.jpg"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn moves_a_file_to_a_free_name() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("Zażółć gęślą jaźń.jpg");
        let to = dir.path().join("Odrzucone – łąka.jpg");
        fs::write(&from, "photo").unwrap();

        move_exclusive(&from, &to).unwrap();

        assert!(!from.exists());
        assert_eq!(fs::read_to_string(&to).unwrap(), "photo");
    }

    #[test]
    fn refuses_an_existing_destination_and_keeps_both_files() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("IMG_1.jpg");
        let to = dir.path().join("IMG_2.jpg");
        fs::write(&from, "moved").unwrap();
        fs::write(&to, "already there").unwrap();

        let err = move_exclusive(&from, &to).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&from).unwrap(), "moved");
        assert_eq!(fs::read_to_string(&to).unwrap(), "already there");
    }

    #[test]
    fn reports_a_missing_source() {
        let dir = TempDir::new().unwrap();

        let err =
            move_exclusive(&dir.path().join("gone.jpg"), &dir.path().join("to.jpg")).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[cfg(unix)]
    #[test]
    fn the_fallback_refuses_an_existing_destination_too() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("IMG_1.jpg");
        let to = dir.path().join("IMG_2.jpg");
        fs::write(&from, "moved").unwrap();
        fs::write(&to, "already there").unwrap();

        let err = rename_after_check(&from, &to).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&to).unwrap(), "already there");
    }
}
