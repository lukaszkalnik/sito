//! Thin Tauri layer: exposes `sito-core` to the UI as commands and events.

mod commands;
mod state;
mod thumbnails;

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
        .manage(state::AppState::default())
        .register_asynchronous_uri_scheme_protocol(thumbnails::SCHEME, |ctx, request, responder| {
            thumbnails::handle(&ctx, &request, responder);
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan::start_scan,
            commands::scan::get_timeline
        ])
        .run(tauri::generate_context!())
}
