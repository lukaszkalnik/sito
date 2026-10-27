//! Thin Tauri layer: exposes `sito-core` to the UI as commands and events.

mod commands;
mod photo_cache;
mod photos;
mod state;
mod thumbnails;
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
                .skip_initial_state(commands::viewer::LABEL)
                .build(),
        )
        .on_window_event(window_fit::save_on_close)
        .manage(state::AppState::default())
        .manage(commands::viewer::ViewerState::default())
        .register_asynchronous_uri_scheme_protocol(thumbnails::SCHEME, |ctx, request, responder| {
            thumbnails::handle(&ctx, &request, responder);
        })
        .register_asynchronous_uri_scheme_protocol(photos::SCHEME, |ctx, request, responder| {
            photos::handle(&ctx, &request, responder);
        })
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let cache_dir = match app.path().app_cache_dir() {
                Ok(dir) => dir,
                Err(err) => {
                    log::warn!("no app cache folder ({err}); using the temp folder");
                    std::env::temp_dir().join("sito")
                }
            };
            app.manage(thumbnails::Thumbnails::new(cache_dir.join("thumbnails"))?);
            if let Err(err) = std::fs::create_dir_all(&cache_dir) {
                log::warn!("cannot create {}: {err}", cache_dir.display());
            }
            app.manage(photo_cache::PhotoCacheState::open(
                &photo_cache::cache_file(&cache_dir),
            ));
            app.manage(commands::reject::Actions::new(open_action_log(
                app.handle(),
            )));
            // The main window is created hidden from the config.
            if let Some(main) = app.get_webview_window("main") {
                window_fit::show_placed(&main);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan::start_scan,
            commands::scan::get_timeline,
            commands::series::get_series,
            commands::viewer::open_viewer,
            commands::viewer::take_viewer_request,
            commands::reject::reject_photo,
            commands::reject::undo_reject,
            commands::reject::get_undo_state
        ])
        .run(tauri::generate_context!())
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
