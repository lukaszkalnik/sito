//! Thin Tauri layer: exposes `sito-core` to the UI as commands and events.

/// Returns the version of the core library.
#[tauri::command]
fn core_version() -> &'static str {
    sito_core::VERSION
}

/// Builds and runs the Tauri application.
///
/// # Errors
///
/// Returns an error if the application fails to start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
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
        .invoke_handler(tauri::generate_handler![core_version])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_version_matches_core_crate() {
        assert_eq!(core_version(), sito_core::VERSION);
    }
}
