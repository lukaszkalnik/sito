//! Moving the cache to a folder the user chooses, or back to the default
//! one (Windows only in the UI; the commands work everywhere, so they are
//! tested everywhere).

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use serde::Serialize;
use sito_core::cache_dir::{
    self, CacheLocation, MoveOutcome, MoveProgress, Surroundings, check_choice,
};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use super::error::CacheError;
use super::gate::CacheGate;
use super::{CacheSettings, CacheSettingsDto, DIR_KEY, save_setting};
use crate::drive;
use crate::error::off_main_thread;
use crate::scan::completion::complete_in_background;
use crate::scan::photo_cache::PhotoCacheState;
use crate::scan::state::AppState;
use crate::throttle::Throttle;
use crate::thumbnails::Thumbnails;

/// How often at most the move's progress is sent: the copy reports after
/// every file, tens of thousands of them.
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

/// A chosen folder that can take the cache, as sent to the UI for the
/// confirmation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckedFolderDto {
    /// The cache folder that would be used (the chosen one's `Sito-cache`).
    target: PathBuf,
    /// Whether it is on a network drive, which can slow Sito down.
    network: bool,
    /// How much the cache to move takes, in bytes.
    size: u64,
}

/// The move's progress, as sent to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveProgressDto {
    /// Files copied so far.
    done: u64,
    /// Files to copy.
    total: u64,
}

impl From<MoveProgress> for MoveProgressDto {
    fn from(progress: MoveProgress) -> Self {
        Self {
            done: progress.done,
            total: progress.total,
        }
    }
}

/// Whether `folder` can take the cache, and what the confirmation shows.
#[tauri::command]
pub async fn check_cache_folder(
    app: AppHandle,
    folder: PathBuf,
) -> Result<CheckedFolderDto, CacheError> {
    off_main_thread(move || {
        let target = checked_target(&app, &folder)?;
        let current = app.state::<CacheSettings>().dir();
        let size = cache_dir::cache_size(&current).unwrap_or_default();
        Ok(CheckedFolderDto {
            network: drive::is_network(&target),
            target,
            size: size.thumbnails.saturating_add(size.photo_data),
        })
    })
    .await?
}

/// The cache folder in `folder`, if it may take the cache.
fn checked_target(app: &AppHandle, folder: &Path) -> Result<PathBuf, CacheError> {
    let settings = app.state::<CacheSettings>();
    let current = settings.dir();
    let open_folder = app
        .state::<AppState>()
        .scan()
        .as_ref()
        .map(|scan| scan.timeline.root.clone());
    Ok(check_choice(
        folder,
        Surroundings {
            current: &current,
            default: &settings.default_dir,
            open_folder: open_folder.as_deref(),
        },
    )?)
}

/// Moves the cache into `folder` (its `Sito-cache` subfolder), or back to
/// the default folder without one, sending progress through
/// `on_progress`. Background work pauses meanwhile and goes on afterwards.
/// Refused while a folder is scanned or refreshed. A failure or
/// [`cancel_cache_move`] leaves the cache where it was.
#[tauri::command]
pub async fn move_cache(
    app: AppHandle,
    folder: Option<PathBuf>,
    on_progress: Channel<MoveProgressDto>,
) -> Result<CacheSettingsDto, CacheError> {
    off_main_thread(move || {
        let target = match &folder {
            Some(folder) => Target::Chosen(checked_target(&app, folder)?),
            None => Target::Default,
        };
        let result = paused(&app, || relocate(&app, &target, &on_progress));
        result.map(|()| app.state::<CacheSettings>().dto())
    })
    .await?
}

/// Stops a move that is copying; it then fails with
/// [`CacheError::Cancelled`].
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn cancel_cache_move(state: State<'_, CacheSettings>) {
    state.cancel.store(true, Ordering::SeqCst);
}

/// Where the cache goes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    /// The default folder.
    Default,
    /// A chosen folder's `Sito-cache`.
    Chosen(PathBuf),
}

/// Runs `work` with the whole cache gate (background work stopped and the
/// photo cache closed), then reopens the photo cache wherever the cache is
/// now and lets the EXIF pass of the shown folder go on.
fn paused(
    app: &AppHandle,
    work: impl FnOnce() -> Result<(), CacheError>,
) -> Result<(), CacheError> {
    let gate = app.state::<CacheGate>();
    let whole = gate.exclusive()?;
    let photo_cache = app.state::<PhotoCacheState>();
    photo_cache.close();
    let result = work();
    let dir = app.state::<CacheSettings>().dir();
    photo_cache.reopen(&cache_dir::photo_cache_file(&dir));
    drop(whole);
    resume_exif_pass(app);
    result
}

/// Starts the EXIF pass of the shown folder again, if it has photos left.
fn resume_exif_pass(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some((root, scan_id)) = state
        .scan()
        .as_ref()
        .map(|scan| (scan.timeline.root.clone(), scan.id))
    else {
        return;
    };
    complete_in_background(app.clone(), root, scan_id, state.latest_generation());
}

/// Moves the cache files to `target`, saves the choice and switches over,
/// then removes the old copy. On failure nothing changes.
fn relocate(
    app: &AppHandle,
    target: &Target,
    on_progress: &Channel<MoveProgressDto>,
) -> Result<(), CacheError> {
    let settings = app.state::<CacheSettings>();
    let from = settings.dir();
    let (to, value) = match target {
        Target::Default => (settings.default_dir.clone(), String::new()),
        // Checked to be valid Unicode.
        Target::Chosen(dir) => (dir.clone(), dir.to_string_lossy().into_owned()),
    };
    if to != from {
        copy_over(&settings, &from, &to, on_progress, || {
            save_setting(app, DIR_KEY, &value)
        })?;
        app.state::<Thumbnails>()
            .move_to(cache_dir::thumbnails_dir(&to));
        if let Err(err) = cache_dir::remove_cache(&from) {
            // The new cache is in use; the old files only take space.
            log::warn!("cannot remove the old cache in {}: {err}", from.display());
        }
    } else if !save_setting(app, DIR_KEY, &value) {
        return Err(CacheError::Internal);
    }
    *settings.location() = match target {
        Target::Default => CacheLocation::Default { dir: to },
        Target::Chosen(_) => CacheLocation::Chosen { dir: to },
    };
    Ok(())
}

/// Moves the cache files from `from` to `to`, then saves the choice with
/// `save`; if that fails, the files go back.
fn copy_over(
    settings: &CacheSettings,
    from: &Path,
    to: &Path,
    on_progress: &Channel<MoveProgressDto>,
    save: impl FnOnce() -> bool,
) -> Result<(), CacheError> {
    settings.cancel.store(false, Ordering::SeqCst);
    let mut throttle = Throttle::new(PROGRESS_EVERY, Instant::now());
    let outcome = cache_dir::move_cache(
        from,
        to,
        || settings.cancel.load(Ordering::SeqCst),
        |progress| {
            if progress.done == progress.total || throttle.is_due(Instant::now()) {
                // A closed channel only means the dialog stopped listening.
                let _ = on_progress.send(progress.into());
            }
        },
    )
    .inspect_err(|err| log::warn!("cannot move the cache to {}: {err}", to.display()))?;
    if save() {
        log::info!("cache moved to {} ({outcome:?})", to.display());
        return Ok(());
    }
    let undone = match outcome {
        MoveOutcome::Renamed => cache_dir::move_cache(to, from, || false, |_| {})
            .map(|_| ())
            .map_err(|err| err.to_string()),
        MoveOutcome::Copied => cache_dir::remove_cache(to).map_err(|err| err.to_string()),
    };
    if let Err(err) = undone {
        log::warn!("cannot undo the move to {}: {err}", to.display());
    }
    Err(CacheError::Internal)
}
