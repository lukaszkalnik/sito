//! Capture times from Google Takeout: the date files Takeout writes next to
//! each photo (`IMG.jpg.supplemental-metadata.json`) hold the time Google
//! Photos knows, including dates the user corrected there.

mod naming;
mod sidecar;
mod time;

pub use naming::match_sidecars;
pub use sidecar::{Sidecar, SidecarError, read_sidecar};
pub use time::{ChosenTime, TakeoutUse, choose_time};
