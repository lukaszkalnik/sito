//! Update check and one-click install: asks GitHub for a newer release of
//! Sito, and on request downloads its installer, verifies its signature
//! against the public key built into the app, and runs it. All network access
//! of the updater happens here, never in the webview.

mod error;

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State, Url};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};
use tauri_plugin_window_state::AppHandleExt;

use crate::error::off_main_thread;
use crate::reject::Actions;
use crate::throttle::Throttle;
use crate::window_fit;
pub use error::UpdateError;

/// Event that tells the main window to check for an update now.
pub const CHECK_REQUESTED_EVENT: &str = "update-check-requested";
/// Environment variable with an endpoint used instead of GitHub's, for testing
/// an update locally. Read only in debug builds.
const ENDPOINT_ENV: &str = "SITO_UPDATE_ENDPOINT";
/// How often at most the download progress is sent to the UI.
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

/// The update found by the last check, so an install installs exactly what
/// the user was shown.
#[derive(Default)]
pub struct UpdateState(Mutex<Option<Update>>);

impl std::fmt::Debug for UpdateState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let version = self.lock().as_ref().map(|update| update.version.clone());
        f.debug_tuple("UpdateState").field(&version).finish()
    }
}

impl UpdateState {
    fn lock(&self) -> MutexGuard<'_, Option<Update>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// How an update is installed on this system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallKind {
    /// Sito downloads and runs the installer.
    InApp,
    /// The user downloads the new version from the releases page. On macOS
    /// until the app is signed (slice 23).
    Manual,
}

impl InstallKind {
    /// The kind of this system.
    const CURRENT: Self = if cfg!(windows) {
        Self::InApp
    } else {
        Self::Manual
    };
}

/// A newer version, as sent to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    /// The release notes, or `None` if the release has none.
    notes: Option<String>,
    install: InstallKind,
}

impl From<&Update> for UpdateInfo {
    fn from(update: &Update) -> Self {
        Self {
            version: update.version.clone(),
            notes: notes_of(update.body.as_deref()),
            install: InstallKind::CURRENT,
        }
    }
}

/// `body` as release notes: `None` if it is missing or blank.
fn notes_of(body: Option<&str>) -> Option<String> {
    body.map(str::trim)
        .filter(|notes| !notes.is_empty())
        .map(str::to_owned)
}

/// The answer of a check, as sent to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    /// The version of the running app.
    current_version: String,
    /// The newer version, or `None` if the running one is the newest.
    update: Option<UpdateInfo>,
}

/// Progress of installing an update, as sent to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum InstallProgress {
    /// `downloaded` of `total` bytes (unknown if the server did not say).
    Downloading { downloaded: u64, total: Option<u64> },
    /// The download is verified; the installer is starting.
    Installing,
}

/// Asks GitHub for a newer release than the running version.
///
/// # Errors
///
/// [`UpdateError::Network`] when GitHub cannot be reached (also offline).
#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    state: State<'_, UpdateState>,
) -> Result<UpdateCheck, UpdateError> {
    let updater = updater(&app).map_err(|err| error::logged("set up the updater", &err))?;
    let update = updater
        .check()
        .await
        .map_err(|err| error::logged("check for an update", &err))?;
    let check = UpdateCheck {
        current_version: app.package_info().version.to_string(),
        update: update.as_ref().map(UpdateInfo::from),
    };
    *state.lock() = update;
    Ok(check)
}

/// Downloads the update found by the last check, reporting the progress to
/// `on_progress`, verifies it and runs its installer. On Windows the app then
/// exits and the installer starts the new version.
///
/// # Errors
///
/// [`UpdateError::Unsupported`] where updates are installed by hand,
/// [`UpdateError::NoUpdate`] without a found update, and the download's and
/// verification's errors.
#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    state: State<'_, UpdateState>,
    on_progress: Channel<InstallProgress>,
) -> Result<(), UpdateError> {
    if InstallKind::CURRENT == InstallKind::Manual {
        return Err(UpdateError::Unsupported);
    }
    let update = state.lock().clone().ok_or(UpdateError::NoUpdate)?;
    let bytes = download(&update, &on_progress).await?;
    send(&on_progress, InstallProgress::Installing);
    off_main_thread(move || {
        // Waits for a running reject or accept and keeps any new one from
        // starting until the process exits, so a move is never cut off
        // between its log entry and the file system. Released if installing
        // fails.
        let actions = app.state::<Actions>();
        let _no_actions = actions.lock();
        update.install(bytes)
    })
    .await
    .map_err(|_| UpdateError::Internal)?
    .map_err(|err| error::logged("install the update", &err))
}

/// Downloads `update`, sending the progress at most every [`PROGRESS_EVERY`].
async fn download(
    update: &Update,
    on_progress: &Channel<InstallProgress>,
) -> Result<Vec<u8>, UpdateError> {
    let mut downloaded: u64 = 0;
    let mut throttle = Throttle::new(PROGRESS_EVERY, Instant::now());
    let on_chunk = |chunk: usize, total: Option<u64>| {
        downloaded = downloaded.saturating_add(u64::try_from(chunk).unwrap_or(u64::MAX));
        if throttle.is_due(Instant::now()) {
            send(
                on_progress,
                InstallProgress::Downloading { downloaded, total },
            );
        }
    };
    update
        .download(on_chunk, || {})
        .await
        .map_err(|err| error::logged("download the update", &err))
}

fn send(channel: &Channel<InstallProgress>, progress: InstallProgress) {
    if let Err(err) = channel.send(progress) {
        log::warn!("cannot send the update progress: {err}");
    }
}

/// The updater, asking the debug endpoint override if one is set.
fn updater(app: &AppHandle) -> tauri_plugin_updater::Result<Updater> {
    let handle = app.clone();
    let mut builder = app
        .updater_builder()
        .on_before_exit(move || before_exit(&handle));
    if let Some(endpoint) = endpoint_override() {
        log::info!("checking for updates at {endpoint}");
        builder = builder.endpoints(vec![endpoint])?;
    }
    builder.build()
}

/// The endpoint from [`ENDPOINT_ENV`] in debug builds. Release builds always
/// ask GitHub; the updater also refuses `http://` there.
fn endpoint_override() -> Option<Url> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let value = std::env::var(ENDPOINT_ENV).ok()?;
    Url::parse(&value)
        .map_err(|err| log::warn!("ignoring {ENDPOINT_ENV}={value}: {err}"))
        .ok()
}

/// Gets the app ready to be closed by the installer, like a close by the user.
/// Replaces the plugin's own hook, which only did the cleanup.
fn before_exit(app: &AppHandle) {
    if let Err(err) = app.save_window_state(window_fit::STATE_FLAGS) {
        log::warn!("cannot save the window size and position: {err}");
    }
    app.cleanup_before_exit();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_release_notes_are_no_notes() {
        assert_eq!(notes_of(None), None);
        assert_eq!(notes_of(Some("  \n")), None);
        assert_eq!(
            notes_of(Some("\n### New\n- Updates\n")),
            Some("### New\n- Updates".to_owned())
        );
    }

    #[test]
    fn progress_is_sent_with_its_kind() {
        assert_eq!(
            serde_json::to_value(InstallProgress::Downloading {
                downloaded: 10,
                total: None,
            })
            .ok(),
            Some(serde_json::json!({ "kind": "downloading", "downloaded": 10, "total": null }))
        );
        assert_eq!(
            serde_json::to_value(InstallProgress::Installing).ok(),
            Some(serde_json::json!({ "kind": "installing" }))
        );
    }
}
