//! Thin Tauri layer: exposes `sito-core` to the UI as commands and events.

mod commands;
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
                Ok(dir) => dir.join("thumbnails"),
                Err(err) => {
                    log::warn!("no app cache folder ({err}); using the temp folder");
                    std::env::temp_dir().join("sito-thumbnails")
                }
            };
            app.manage(thumbnails::Thumbnails::new(cache_dir)?);
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
            commands::viewer::take_viewer_request
        ])
        .run(tauri::generate_context!())
}
