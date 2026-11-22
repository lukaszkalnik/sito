//! Paths in the form Windows takes beyond 260 characters.

use std::io;
use std::path::{Path, PathBuf};

/// `path` in its verbatim form (`\\?\C:\...`, `\\?\UNC\server\share\...`),
/// absolute and with `.` and `..` resolved, which Windows does not limit to
/// 260 characters.
///
/// # Errors
///
/// Returns an error if the path is empty or cannot be made absolute.
#[cfg(windows)]
pub(crate) fn verbatim(path: &Path) -> io::Result<PathBuf> {
    use std::ffi::OsString;
    use std::path::{Component, Prefix};

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
    Ok(PathBuf::from(text))
}

/// `path` as it is: other systems have no such limit.
///
/// # Errors
///
/// Never; the signature matches the Windows one.
#[cfg(not(windows))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the same signature as on Windows, where it can fail"
)]
pub(crate) fn verbatim(path: &Path) -> io::Result<PathBuf> {
    Ok(path.to_path_buf())
}
