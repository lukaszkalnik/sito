//! The cache settings („Ustawienia → Pamięć podręczna…”): where the cache
//! is, how much space thumbnails may take, how much the cache takes now,
//! emptying it, and (on Windows) moving it to another folder.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use sito_core::actions::ActionLog;
use sito_core::cache_dir::{self, CacheLocation};
use tauri::{AppHandle, Manager, State};

use crate::error::off_main_thread;
use crate::reject::Actions;
use crate::scan::photo_cache::PhotoCacheState;
use crate::thumbnails::Thumbnails;
use error::CacheError;
use gate::CacheGate;

pub mod error;
pub mod gate;
pub mod relocation;

/// The setting with the cache folder the user chose; empty or missing for
/// the default folder.
const DIR_KEY: &str = "cache_dir";
/// The setting with the space thumbnails may take, in MB.
const LIMIT_KEY: &str = "thumbnail_limit_mb";
/// The space thumbnails may take unless the user chose another, in MB.
pub const DEFAULT_LIMIT_MB: u64 = 500;
/// The least space thumbnails may be given, in MB: room for a few thousand.
pub const MIN_LIMIT_MB: u64 = 50;
const MB: u64 = 1024 * 1024;

/// Where the cache is and how large it may grow.
#[derive(Debug)]
pub struct CacheSettings {
    default_dir: PathBuf,
    location: Mutex<CacheLocation>,
    limit_mb: AtomicU64,
    /// Set by „Anuluj” while a move copies.
    cancel: AtomicBool,
}

impl CacheSettings {
    /// The settings saved in `log` (the defaults without one), with the
    /// cache in `default_dir` unless the user chose another folder that is
    /// there.
    pub fn load(log: Option<&ActionLog>, default_dir: PathBuf) -> Self {
        let setting = |key| {
            log.and_then(|log| {
                log.setting(key)
                    .map_err(|err| log::warn!("cannot read the setting {key}: {err}"))
                    .ok()
                    .flatten()
            })
        };
        let chosen = setting(DIR_KEY)
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from);
        let location = CacheLocation::resolve(chosen, default_dir.clone(), Path::is_dir);
        if let CacheLocation::Fallback { chosen, .. } = &location {
            log::warn!(
                "cache folder {} is missing; using the default",
                chosen.display()
            );
        }
        Self {
            default_dir,
            location: Mutex::new(location),
            limit_mb: AtomicU64::new(parse_limit(setting(LIMIT_KEY).as_deref())),
            cancel: AtomicBool::new(false),
        }
    }

    fn location(&self) -> MutexGuard<'_, CacheLocation> {
        self.location.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The cache folder in use.
    pub fn dir(&self) -> PathBuf {
        self.location().dir().to_path_buf()
    }

    /// The space thumbnails may take, in bytes.
    pub fn limit_bytes(&self) -> u64 {
        self.limit_mb.load(Ordering::Relaxed).saturating_mul(MB)
    }

    fn dto(&self) -> CacheSettingsDto {
        CacheSettingsDto {
            folder: self.dir(),
            location: LocationDto::from(&*self.location()),
            platform: PlatformDto::current(),
            thumbnail_limit_mb: self.limit_mb.load(Ordering::Relaxed),
        }
    }
}

/// The limit saved as `value`, at least [`MIN_LIMIT_MB`]; the default if
/// none is saved or it is not a number.
fn parse_limit(value: Option<&str>) -> u64 {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .map_or(DEFAULT_LIMIT_MB, |mb| mb.max(MIN_LIMIT_MB))
}

/// Where the cache is, as sent to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LocationDto {
    /// The system's app cache folder.
    Default,
    /// A folder the user chose.
    Chosen,
    /// The chosen folder is missing; the default is used this session.
    Fallback {
        /// The chosen folder.
        chosen: PathBuf,
    },
}

impl From<&CacheLocation> for LocationDto {
    fn from(location: &CacheLocation) -> Self {
        match location {
            CacheLocation::Default { .. } => Self::Default,
            CacheLocation::Chosen { .. } => Self::Chosen,
            CacheLocation::Fallback { chosen, .. } => Self::Fallback {
                chosen: chosen.clone(),
            },
        }
    }
}

/// The system Sito runs on: the cache can be moved on Windows only; elsewhere
/// its folder is shown in the system's file manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlatformDto {
    /// Windows.
    Windows,
    /// macOS, whose file manager is the Finder.
    Macos,
    /// Linux and others.
    Other,
}

impl PlatformDto {
    /// The system this build is for.
    const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Other
        }
    }
}

/// The cache settings, as sent to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheSettingsDto {
    /// The cache folder in use.
    folder: PathBuf,
    location: LocationDto,
    platform: PlatformDto,
    /// The space thumbnails may take, in MB.
    thumbnail_limit_mb: u64,
}

/// How much space the cache takes, as sent to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheSizeDto {
    /// Thumbnails and display images, in bytes.
    thumbnails: u64,
    /// The photo cache, in bytes.
    photo_data: u64,
}

/// The cache settings.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn get_cache_settings(state: State<'_, CacheSettings>) -> CacheSettingsDto {
    state.dto()
}

/// How much space the cache takes now; summing a large thumbnail folder
/// takes a moment.
#[tauri::command]
pub async fn measure_cache(app: AppHandle) -> Result<CacheSizeDto, CacheError> {
    off_main_thread(move || {
        let dir = app.state::<CacheSettings>().dir();
        cache_dir::cache_size(&dir)
            .map(|size| CacheSizeDto {
                thumbnails: size.thumbnails,
                photo_data: size.photo_data,
            })
            .map_err(|err| {
                log::warn!("cannot measure the cache in {}: {err}", dir.display());
                CacheError::Internal
            })
    })
    .await?
}

/// Lets thumbnails take `mb` MB (at least [`MIN_LIMIT_MB`]), saves it, and
/// cleans the thumbnail cache down to it in the background. Returns the
/// limit now in effect.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn set_thumbnail_limit(app: AppHandle, mb: u64, state: State<'_, CacheSettings>) -> u64 {
    let mb = mb.max(MIN_LIMIT_MB);
    let previous = state.limit_mb.swap(mb, Ordering::Relaxed);
    save_setting(&app, LIMIT_KEY, &mb.to_string());
    if mb < previous {
        clean_thumbnails_in_background(app);
    }
    mb
}

/// Removes every cached thumbnail; they are rendered again as needed.
#[tauri::command]
pub async fn clear_thumbnails(app: AppHandle) -> Result<(), CacheError> {
    off_main_thread(move || {
        let _share = app.state::<CacheGate>().inner().work();
        let cache = app.state::<Thumbnails>().cache();
        match cache.clear() {
            Ok(removed) => {
                log::info!("removed {removed} cached thumbnails");
                Ok(())
            }
            Err(err) => {
                log::warn!("cannot empty {}: {err}", cache.dir().display());
                Err(CacheError::Internal)
            }
        }
    })
    .await?
}

/// Empties the photo cache: every folder is read in full on its next
/// opening. Refused while a folder is scanned.
#[tauri::command]
pub async fn clear_photo_data(app: AppHandle) -> Result<(), CacheError> {
    off_main_thread(move || {
        let gate = app.state::<CacheGate>();
        let _whole = gate.exclusive()?;
        app.state::<PhotoCacheState>().clear().map_err(|err| {
            log::warn!("cannot empty the photo cache: {err}");
            CacheError::Internal
        })
    })
    .await?
}

/// Shows the cache folder in the system's file manager.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn reveal_cache_folder(state: State<'_, CacheSettings>) -> Result<(), CacheError> {
    let dir = state.dir();
    // The opener's free function, not its plugin: the webview gets no
    // permission to open paths itself.
    tauri_plugin_opener::open_path(&dir, None::<&str>).map_err(|err| {
        log::warn!("cannot show {}: {err}", dir.display());
        CacheError::RevealFailed
    })
}

/// Cleans the thumbnail cache down to the limit (see
/// [`PhotoCacheState::clean_thumbnails`]), waiting while the cache moves.
pub fn clean_thumbnails(app: &AppHandle) {
    let _share = app.state::<CacheGate>().inner().work();
    let limit = app.state::<CacheSettings>().limit_bytes();
    app.state::<PhotoCacheState>()
        .clean_thumbnails(&app.state::<Thumbnails>(), limit);
}

/// [`clean_thumbnails`] on a blocking thread.
pub fn clean_thumbnails_in_background(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(move || clean_thumbnails(&app));
}

/// Saves the setting `key`; `false` (with a warning) if it cannot be saved.
fn save_setting(app: &AppHandle, key: &str, value: &str) -> bool {
    let actions = app.state::<Actions>().inner().lock();
    let Some(log) = actions.opened_log() else {
        log::warn!("no action log; the setting {key} is not saved");
        return false;
    };
    log.set_setting(key, value)
        .map_err(|err| log::warn!("cannot save the setting {key}: {err}"))
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log_with(settings: &[(&str, &str)]) -> ActionLog {
        let log = ActionLog::open_in_memory().unwrap();
        for (key, value) in settings {
            log.set_setting(key, value).unwrap();
        }
        log
    }

    #[test]
    fn without_settings_the_default_folder_and_limit_are_used() {
        let default = tempfile::tempdir().unwrap();
        let settings = CacheSettings::load(None, default.path().to_path_buf());

        assert_eq!(settings.dir(), default.path());
        assert_eq!(settings.dto().location, LocationDto::Default);
        assert_eq!(settings.limit_bytes(), 500 * MB);
    }

    #[test]
    fn a_chosen_folder_that_exists_is_used() {
        let (default, chosen) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let log = log_with(&[(DIR_KEY, chosen.path().to_str().unwrap())]);

        let settings = CacheSettings::load(Some(&log), default.path().to_path_buf());

        assert_eq!(settings.dir(), chosen.path());
        assert_eq!(settings.dto().location, LocationDto::Chosen);
    }

    #[test]
    fn a_missing_chosen_folder_falls_back_and_the_setting_stays() {
        let default = tempfile::tempdir().unwrap();
        let missing = default.path().join("unplugged").join("Sito-cache");
        let log = log_with(&[(DIR_KEY, missing.to_str().unwrap())]);

        let settings = CacheSettings::load(Some(&log), default.path().to_path_buf());

        assert_eq!(settings.dir(), default.path());
        assert_eq!(
            settings.dto().location,
            LocationDto::Fallback { chosen: missing }
        );
        assert!(log.setting(DIR_KEY).unwrap().is_some());
    }

    #[test]
    fn an_empty_choice_is_the_default() {
        let default = tempfile::tempdir().unwrap();
        let log = log_with(&[(DIR_KEY, "")]);

        let settings = CacheSettings::load(Some(&log), default.path().to_path_buf());

        assert_eq!(settings.dto().location, LocationDto::Default);
    }

    #[test]
    fn a_saved_limit_is_used_and_never_below_the_least() {
        assert_eq!(parse_limit(Some("2000")), 2000);
        assert_eq!(parse_limit(Some("10")), MIN_LIMIT_MB);
        assert_eq!(parse_limit(Some("lots")), DEFAULT_LIMIT_MB);
        assert_eq!(parse_limit(None), DEFAULT_LIMIT_MB);
    }

    #[test]
    fn settings_are_sent_with_their_location_kind() {
        let value = serde_json::to_value(LocationDto::Fallback {
            chosen: PathBuf::from("D:/Sito-cache"),
        })
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!({ "kind": "fallback", "chosen": "D:/Sito-cache" })
        );
    }
}
