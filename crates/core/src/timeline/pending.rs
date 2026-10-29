//! Completing photos whose EXIF was left unread by a quick first pass.

use super::Photo;
use super::read::{READ_CHUNK, to_photo};
use crate::scan::FoundFile;

/// How [`complete_pending`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    /// Every pending photo was read.
    Finished,
    /// Stopped early because `should_stop` said so; the rest stay pending.
    Stopped,
}

/// Reads the EXIF of `pending` photos one at a time on the calling thread,
/// in their order, reporting `(done, total)` to `on_progress` after each
/// photo. The photos read are handed to `store` in chunks of a few hundred.
/// Before each chunk, `should_stop` may end the work; the photos not read
/// yet stay pending.
///
/// It never uses the rayon pool, so image decoding (which does) is not held
/// up while it runs.
pub fn complete_pending(
    pending: &[Photo],
    mut store: impl FnMut(&[Photo]),
    mut on_progress: impl FnMut(usize, usize),
    should_stop: impl Fn() -> bool,
) -> Completion {
    let total = pending.len();
    let mut done = 0;
    for chunk in pending.chunks(READ_CHUNK) {
        if should_stop() {
            return Completion::Stopped;
        }
        // Sequential on purpose: this runs while the timeline is in use, and
        // parallel reads would fill the rayon pool that decodes thumbnails
        // (and are no faster on a hard disk).
        let read: Vec<Photo> = chunk
            .iter()
            .map(|photo| {
                let read = to_photo(&found(photo));
                done += 1;
                on_progress(done, total);
                read
            })
            .collect();
        store(&read);
    }
    Completion::Finished
}

/// The file of `photo` as a scan found it.
fn found(photo: &Photo) -> FoundFile {
    FoundFile {
        path: photo.path.clone(),
        size: photo.size,
        modified: photo.modified,
        sidecar: photo.sidecar.clone(),
        companion: photo.companion.clone(),
    }
}
