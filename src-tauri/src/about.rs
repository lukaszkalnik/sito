//! The app menu with „Pomoc → Licencje…”, which shows the third-party notices
//! in the main window.

use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::{AppHandle, Emitter, WebviewWindow};

/// Event that tells the main window to show the licenses dialog.
pub const SHOW_LICENSES_EVENT: &str = "about-show-licenses";
/// Label of the window that shows the licenses.
const MAIN_LABEL: &str = "main";
/// Menu id of „Licencje…”.
const LICENSES_ID: &str = "licenses";
// Native menus cannot use the UI's translation file; slice 22 moves these
// strings into the translations.
const LICENSES_LABEL: &str = "Licencje…";
#[cfg(not(target_os = "macos"))]
const HELP_LABEL: &str = "Pomoc";

/// What a click on one of Sito's menu items does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    /// Show the third-party notices.
    ShowLicenses,
}

impl MenuAction {
    /// The action of the menu item with `id`, or `None` for items the system
    /// handles itself (Copy, Quit, ...).
    fn of(id: &str) -> Option<Self> {
        (id == LICENSES_ID).then_some(Self::ShowLicenses)
    }
}

/// Adds the menu to the main window. On macOS, where the menu is app-wide,
/// „Licencje…” goes into the Help menu of the default menu instead, so the
/// standard items (Copy, Paste, Quit) stay.
///
/// # Errors
///
/// Returns an error if the menu cannot be created or set.
pub fn install_menu(app: &AppHandle, main: &WebviewWindow) -> tauri::Result<()> {
    let licenses = MenuItem::with_id(app, LICENSES_ID, LICENSES_LABEL, true, None::<&str>)?;
    set_menu(app, main, &licenses)
}

#[cfg(target_os = "macos")]
fn set_menu(
    app: &AppHandle,
    _main: &WebviewWindow,
    licenses: &MenuItem<tauri::Wry>,
) -> tauri::Result<()> {
    let menu = Menu::default(app)?;
    if let Some(help) = menu
        .get(tauri::menu::HELP_SUBMENU_ID)
        .and_then(|item| item.as_submenu().cloned())
    {
        help.append(licenses)?;
    }
    app.set_menu(menu)?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn set_menu(
    app: &AppHandle,
    main: &WebviewWindow,
    licenses: &MenuItem<tauri::Wry>,
) -> tauri::Result<()> {
    let help = tauri::menu::Submenu::with_items(app, HELP_LABEL, true, &[licenses])?;
    main.set_menu(Menu::with_items(app, &[&help])?)?;
    Ok(())
}

/// Runs the action of a clicked menu item.
pub fn on_menu_event(app: &AppHandle, event: &MenuEvent) {
    let Some(action) = MenuAction::of(event.id().as_ref()) else {
        return;
    };
    match action {
        MenuAction::ShowLicenses => {
            if let Err(err) = app.emit_to(MAIN_LABEL, SHOW_LICENSES_EVENT, ()) {
                log::warn!("cannot show the licenses: {err}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn licenses_item_shows_the_licenses() {
        assert_eq!(MenuAction::of(LICENSES_ID), Some(MenuAction::ShowLicenses));
    }

    #[test]
    fn system_items_have_no_action() {
        assert_eq!(MenuAction::of("quit"), None);
        assert_eq!(MenuAction::of(""), None);
    }
}
