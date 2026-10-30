//! Whether a folder's drive has a trash.
//!
//! On Windows, removable and network drives have no Recycle Bin: moving a
//! file "to the trash" there deletes it for good. Only fixed drives qualify.

#![cfg_attr(
    windows,
    expect(
        unsafe_code,
        reason = "one call into kernel32 to read the drive type; std has no API for it"
    )
)]

use std::path::Path;

/// Whether files in `folder` go into a trash they can be restored from.
#[cfg(windows)]
pub(super) fn has_trash(folder: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;

    /// `DRIVE_FIXED` from `fileapi.h`.
    const DRIVE_FIXED: u32 = 3;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDriveTypeW(root_path_name: *const u16) -> u32;
    }

    // The last ancestor is the drive's root (`C:\`, `\\server\share\`), the
    // form the call expects.
    let Some(root) = folder.ancestors().last() else {
        return false;
    };
    let root: Vec<u16> = root.as_os_str().encode_wide().chain([0]).collect();
    // SAFETY: `root` is a NUL-terminated UTF-16 string that outlives the call,
    // which only reads it.
    let drive_type = unsafe { GetDriveTypeW(root.as_ptr()) };
    drive_type == DRIVE_FIXED
}

/// Whether files in `folder` go into a trash they can be restored from:
/// macOS keeps a trash on every writable volume.
#[cfg(not(windows))]
pub(super) fn has_trash(_folder: &Path) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_temp_folder_has_a_trash() {
        // The temp folder is on the system's own (fixed) drive.
        assert!(has_trash(&std::env::temp_dir()));
    }

    #[cfg(windows)]
    #[test]
    fn a_drive_that_does_not_exist_has_none() {
        // Picks a letter not mapped on this machine.
        let free = (b'D'..=b'Z')
            .map(|letter| format!("{}:\\", char::from(letter)))
            .find(|root| !Path::new(root).exists());
        if let Some(root) = free {
            assert!(!has_trash(Path::new(&root).join("Zdjęcia").as_path()));
        }
    }
}
