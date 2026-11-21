//! The app menu: „Pomoc → Sprawdź aktualizacje…”, which checks for a newer
//! version, and „Pomoc → Licencje…”, which shows the third-party notices, both
//! in the main window.

use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::{AppHandle, Emitter, WebviewWindow};

use crate::update::CHECK_REQUESTED_EVENT;

/// Event that tells the main window to show the licenses dialog.
pub const SHOW_LICENSES_EVENT: &str = "about-show-licenses";
/// Label of the window that shows the licenses and the update notice.
const MAIN_LABEL: &str = "main";
/// Menu id of „Licencje…”.
const LICENSES_ID: &str = "licenses";
/// Menu id of „Sprawdź aktualizacje…”.
const CHECK_FOR_UPDATES_ID: &str = "check-for-updates";
// Native menus cannot use the UI's translation file; slice 22 moves these
// strings into the translations.
const LICENSES_LABEL: &str = "Licencje…";
const CHECK_FOR_UPDATES_LABEL: &str = "Sprawdź aktualizacje…";
#[cfg(not(target_os = "macos"))]
const HELP_LABEL: &str = "Pomoc";

/// What a click on one of Sito's menu items does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    /// Show the third-party notices.
    ShowLicenses,
    /// Check for a newer version and say what was found.
    CheckForUpdates,
}

impl MenuAction {
    /// The action of the menu item with `id`, or `None` for items the system
    /// handles itself (Copy, Quit, ...).
    fn of(id: &str) -> Option<Self> {
        match id {
            LICENSES_ID => Some(Self::ShowLicenses),
            CHECK_FOR_UPDATES_ID => Some(Self::CheckForUpdates),
            _ => None,
        }
    }
}

/// Adds the menu to the main window. On macOS, where the menu is app-wide,
/// Sito's items go into the Help menu of the default menu instead, so the
/// standard items (Copy, Paste, Quit) stay.
///
/// # Errors
///
/// Returns an error if the menu cannot be created or set.
pub fn install_menu(app: &AppHandle, main: &WebviewWindow) -> tauri::Result<()> {
    let updates = MenuItem::with_id(
        app,
        CHECK_FOR_UPDATES_ID,
        CHECK_FOR_UPDATES_LABEL,
        true,
        None::<&str>,
    )?;
    let licenses = MenuItem::with_id(app, LICENSES_ID, LICENSES_LABEL, true, None::<&str>)?;
    set_menu(app, main, &[&updates, &licenses])
}

#[cfg(target_os = "macos")]
fn set_menu(
    app: &AppHandle,
    _main: &WebviewWindow,
    items: &[&MenuItem<tauri::Wry>],
) -> tauri::Result<()> {
    let menu = Menu::default(app)?;
    if let Some(help) = menu
        .get(tauri::menu::HELP_SUBMENU_ID)
        .and_then(|item| item.as_submenu().cloned())
    {
        for item in items {
            help.append(*item)?;
        }
    }
    app.set_menu(menu)?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn set_menu(
    app: &AppHandle,
    main: &WebviewWindow,
    items: &[&MenuItem<tauri::Wry>],
) -> tauri::Result<()> {
    let items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = items
        .iter()
        .map(|item| *item as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
        .collect();
    let help = tauri::menu::Submenu::with_items(app, HELP_LABEL, true, &items)?;
    main.set_menu(Menu::with_items(app, &[&help])?)?;
    Ok(())
}

/// Runs the action of a clicked menu item.
pub fn on_menu_event(app: &AppHandle, event: &MenuEvent) {
    let Some(action) = MenuAction::of(event.id().as_ref()) else {
        return;
    };
    let event = match action {
        MenuAction::ShowLicenses => SHOW_LICENSES_EVENT,
        MenuAction::CheckForUpdates => CHECK_REQUESTED_EVENT,
    };
    if let Err(err) = app.emit_to(MAIN_LABEL, event, ()) {
        log::warn!("cannot send {event} to the main window: {err}");
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
    fn check_for_updates_item_checks_for_updates() {
        assert_eq!(
            MenuAction::of(CHECK_FOR_UPDATES_ID),
            Some(MenuAction::CheckForUpdates)
        );
    }

    #[test]
    fn system_items_have_no_action() {
        assert_eq!(MenuAction::of("quit"), None);
        assert_eq!(MenuAction::of(""), None);
    }
}
