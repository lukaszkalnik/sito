//! The second pass of a quick first scan: reading the EXIF that the first
//! pass left pending, in the background while the timeline is in use.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Serialize;
use sito_core::timeline::{Completion, Photo, complete_pending};
use tauri::{AppHandle, Emitter, Manager};

use super::photo_cache::PhotoCacheState;
use super::state::AppState;
use super::{SCAN_REFRESHED, ScanSummaryDto};
use crate::reject::TIMELINE_CHANGED;

/// Event with the [`CompletingDto`] progress of the second pass.
pub const SCAN_COMPLETING: &str = "scan-completing";

/// How often at most the timeline is updated while the second pass runs:
/// every update makes the UI reload the timeline and its series.
const UPDATE_EVERY: Duration = Duration::from_secs(10);

/// Progress of the second pass, as sent to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletingDto {
    /// The scan whose photos are being completed.
    scan_id: u64,
    /// Photos read so far.
    done: usize,
    /// Photos to read in total.
    total: usize,
}

/// Reads the pending EXIF of the photos of `root` (shown as scan `scan_id`)
/// on a blocking thread, updating that scan as it goes. Stops when another
/// scan replaces it; the photos not read yet stay pending in the cache.
pub fn complete_in_background(app: AppHandle, root: PathBuf, scan_id: u64) {
    tauri::async_runtime::spawn_blocking(move || {
        let pending = app.state::<PhotoCacheState>().pending(&root);
        if pending.is_empty() {
            return;
        }
        log::info!("reading EXIF of {} photos in the background", pending.len());
        complete(&app, &pending, scan_id);
    });
}

/// Runs the second pass over `pending`, applying what it read to scan
/// `scan_id` at most every [`UPDATE_EVERY`] and in full once it finishes.
fn complete(app: &AppHandle, pending: &[Photo], scan_id: u64) {
    let cache = app.state::<PhotoCacheState>();
    let state = app.state::<AppState>();
    let mut unapplied: Vec<Photo> = Vec::new();
    let mut throttle = Throttle::new(UPDATE_EVERY, Instant::now());
    let completion = complete_pending(
        pending,
        |read| {
            cache.store(read);
            unapplied.extend_from_slice(read);
            if throttle.is_due(Instant::now())
                && state.update_photos(scan_id, &std::mem::take(&mut unapplied))
            {
                emit(app, TIMELINE_CHANGED, ());
            }
        },
        |done, total| {
            emit(
                app,
                SCAN_COMPLETING,
                CompletingDto {
                    scan_id,
                    done,
                    total,
                },
            );
        },
        || !state.is_current(scan_id),
    );
    match completion {
        Completion::Finished if state.update_photos(scan_id, &unapplied) => {
            announce_finished(app, scan_id);
        }
        // Stopped because another scan replaced this one: nothing to show.
        Completion::Finished | Completion::Stopped => {}
    }
}

/// Tells the UI that the timeline of scan `scan_id` is complete.
fn announce_finished(app: &AppHandle, scan_id: u64) {
    let state = app.state::<AppState>();
    let summary = {
        let scan = state.scan();
        match scan.as_ref() {
            Some(scan) if scan.id == scan_id => ScanSummaryDto::new(scan_id, &scan.timeline),
            _ => return,
        }
    };
    emit(app, TIMELINE_CHANGED, ());
    emit(app, SCAN_REFRESHED, summary);
}

fn emit(app: &AppHandle, event: &str, payload: impl Serialize + Clone) {
    if let Err(err) = app.emit(event, payload) {
        log::warn!("cannot send {event}: {err}");
    }
}

/// Says when at least `every` has passed since it last said so.
#[derive(Debug)]
struct Throttle {
    every: Duration,
    last: Instant,
}

impl Throttle {
    fn new(every: Duration, start: Instant) -> Self {
        Self { every, last: start }
    }

    /// Whether `every` has passed since the start or the last `true`.
    fn is_due(&mut self, now: Instant) -> bool {
        let due = now.duration_since(self.last) >= self.every;
        if due {
            self.last = now;
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_throttle_is_due_once_per_period() {
        let start = Instant::now();
        let mut throttle = Throttle::new(Duration::from_secs(10), start);

        assert!(!throttle.is_due(start + Duration::from_secs(9)));
        assert!(throttle.is_due(start + Duration::from_secs(10)));
        assert!(!throttle.is_due(start + Duration::from_secs(19)));
        assert!(throttle.is_due(start + Duration::from_secs(21)));
    }
}
