//! Thin Tauri layer: exposes `sito-core` to the UI as commands and events.

mod about;
mod error;
mod protocol;
mod reject;
mod scan;
mod series;
mod smoke;
mod thumbnails;
mod viewer;
mod window_fit;

use tauri::Manager;

/// Builds and runs the Tauri application.
///
/// # Errors
///
/// Returns an error if the application fails to start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Remembers the size and position of both windows across launches. The
        // windows are placed by `window_fit::show_placed` instead of the plugin, whose
        // own restore runs later on the main thread and would move a shown window.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(window_fit::STATE_FLAGS)
                .skip_initial_state("main")
                .skip_initial_state(viewer::LABEL)
                .build(),
        )
        .on_window_event(window_fit::save_on_close)
        .on_menu_event(|app, event| about::on_menu_event(app, &event))
        .manage(scan::state::AppState::default())
        .manage(viewer::ViewerState::default())
        .register_asynchronous_uri_scheme_protocol(thumbnails::SCHEME, |ctx, request, responder| {
            thumbnails::handle(&ctx, &request, responder);
        })
        .register_asynchronous_uri_scheme_protocol(
            viewer::photo_protocol::SCHEME,
            |ctx, request, responder| {
                viewer::photo_protocol::handle(&ctx, &request, responder);
            },
        )
        .setup(|app| setup(app))
        .invoke_handler(tauri::generate_handler![
            scan::start_scan,
            scan::get_timeline,
            series::get_series,
            viewer::open_viewer,
            viewer::take_viewer_request,
            reject::reject_photo,
            reject::undo_reject,
            reject::get_undo_state
        ])
        .run(tauri::generate_context!())
}

/// Registers the logger, opens the caches and the action log, and shows the
/// main window with its menu.
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(debug_assertions) {
        app.handle().plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )?;
    }
    let cache_dir = app_cache_dir(app.handle());
    app.manage(thumbnails::Thumbnails::new(cache_dir.join("thumbnails"))?);
    if let Err(err) = std::fs::create_dir_all(&cache_dir) {
        log::warn!("cannot create {}: {err}", cache_dir.display());
    }
    app.manage(scan::photo_cache::PhotoCacheState::open(
        &scan::photo_cache::cache_file(&cache_dir),
    ));
    app.manage(reject::Actions::new(open_action_log(app.handle())));
    // The main window is created hidden from the config.
    if let Some(main) = app.get_webview_window("main") {
        // Without the menu the app still works; only the licenses are out of reach.
        if let Err(err) = about::install_menu(app.handle(), &main) {
            log::warn!("cannot create the menu: {err}");
        }
        window_fit::show_placed(&main);
    }
    if smoke::enabled(std::env::var_os(smoke::ENV).as_deref()) {
        smoke::spawn(app.handle().clone());
    }
    Ok(())
}

/// The app cache folder, or a `sito` folder in the temp folder (with a
/// warning) if the system has none.
fn app_cache_dir(app: &tauri::AppHandle) -> std::path::PathBuf {
    app.path().app_cache_dir().unwrap_or_else(|err| {
        log::warn!("no app cache folder ({err}); using the temp folder");
        std::env::temp_dir().join("sito")
    })
}

/// Opens the log of file operations in the app data folder, or `None` (with a
/// warning) if that fails; rejecting photos is then refused.
fn open_action_log(app: &tauri::AppHandle) -> Option<sito_core::actions::ActionLog> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| log::warn!("no app data folder: {err}"))
        .ok()?;
    std::fs::create_dir_all(&dir)
        .map_err(|err| log::warn!("cannot create {}: {err}", dir.display()))
        .ok()?;
    sito_core::actions::ActionLog::open(&dir.join("sito.db"))
        .map_err(|err| log::warn!("cannot open the action log: {err}"))
        .ok()
}
