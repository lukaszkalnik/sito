//! Thin Tauri layer: exposes `sito-core` to the UI as commands and events.

mod commands;
mod state;

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
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan::start_scan,
            commands::scan::get_timeline
        ])
        .run(tauri::generate_context!())
}
