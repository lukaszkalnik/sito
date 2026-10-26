//! State shared between commands.

use std::sync::{Mutex, MutexGuard, PoisonError};

use sito_core::timeline::Timeline;

/// App-wide state managed by Tauri.
#[derive(Debug, Default)]
pub struct AppState {
    timeline: Mutex<Option<Timeline>>,
}

impl AppState {
    /// The timeline of the last finished scan, if any.
    pub fn timeline(&self) -> MutexGuard<'_, Option<Timeline>> {
        // A panic while holding the lock cannot leave the timeline half-written
        // (it is only ever replaced as a whole), so a poisoned lock is safe to use.
        self.timeline.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Replaces the stored timeline.
    pub fn set_timeline(&self, timeline: Timeline) {
        *self.timeline() = Some(timeline);
    }
}
