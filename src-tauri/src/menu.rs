//! The app menu, in the UI language: „Ustawienia → Język”, which chooses the
//! language, „Ustawienia → Pamięć podręczna…”, which shows the cache
//! settings in the main window, and „Pomoc → Sprawdź aktualizacje…” and „Pomoc → Licencje…”,
//! which check for a newer version and show the third-party notices in the
//! main window. It is rebuilt when the language changes.

mod labels;

use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow, Wry};

use crate::language::{Language, LanguageChoice, LanguageState, change_language};
use crate::update::CHECK_REQUESTED_EVENT;
use labels::{MenuLabels, own_name};

/// Event that tells the main window to show the licenses dialog.
pub const SHOW_LICENSES_EVENT: &str = "about-show-licenses";
/// Event that tells the main window to show the cache settings.
pub const SHOW_CACHE_SETTINGS_EVENT: &str = "settings-show-cache";
/// Label of the window that shows the licenses and the update notice.
const MAIN_LABEL: &str = "main";
/// Menu id of „Licencje…”.
const LICENSES_ID: &str = "licenses";
/// Menu id of „Pamięć podręczna…”.
const CACHE_SETTINGS_ID: &str = "cache-settings";
/// Menu id of „Sprawdź aktualizacje…”.
const CHECK_FOR_UPDATES_ID: &str = "check-for-updates";
/// Menu ids of the language choices, in the order the menu shows them.
const LANGUAGE_IDS: [(&str, LanguageChoice); 4] = [
    ("language-system", LanguageChoice::System),
    ("language-pl", LanguageChoice::Fixed(Language::Polish)),
    ("language-en", LanguageChoice::Fixed(Language::English)),
    ("language-de", LanguageChoice::Fixed(Language::German)),
];

/// What a click on one of Sito's menu items does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    /// Show the third-party notices.
    ShowLicenses,
    /// Show the cache settings.
    ShowCacheSettings,
    /// Check for a newer version and say what was found.
    CheckForUpdates,
    /// Use this language.
    SetLanguage(LanguageChoice),
}

impl MenuAction {
    /// The action of the menu item with `id`, or `None` for items the system
    /// handles itself (Copy, Quit, ...).
    fn of(id: &str) -> Option<Self> {
        match id {
            LICENSES_ID => Some(Self::ShowLicenses),
            CACHE_SETTINGS_ID => Some(Self::ShowCacheSettings),
            CHECK_FOR_UPDATES_ID => Some(Self::CheckForUpdates),
            _ => LANGUAGE_IDS
                .iter()
                .find(|(item, _)| *item == id)
                .map(|(_, choice)| Self::SetLanguage(*choice)),
        }
    }
}

/// The menu items Sito adds.
struct SitoItems {
    /// „Ustawienia” with its „Język” submenu and „Pamięć podręczna…”.
    settings: Submenu<Wry>,
    /// „Sprawdź aktualizacje…” and „Licencje…”, for the Help menu.
    help: [MenuItem<Wry>; 2],
}

/// Sets the menu of the main window in the current language. On macOS,
/// where the menu is app-wide, Sito's items go into the default menu
/// instead (settings into the app menu, the rest into Help), so the standard
/// items (Copy, Paste, Quit) stay.
///
/// # Errors
///
/// Returns an error if the menu cannot be created or set.
pub fn install_menu(app: &AppHandle, main: &WebviewWindow) -> tauri::Result<()> {
    let current = app.state::<LanguageState>().current();
    let labels = MenuLabels::of(current.language);
    let items = SitoItems {
        settings: settings_menu(app, &labels, current.choice)?,
        help: [
            MenuItem::with_id(
                app,
                CHECK_FOR_UPDATES_ID,
                labels.check_for_updates,
                true,
                None::<&str>,
            )?,
            MenuItem::with_id(app, LICENSES_ID, labels.licenses, true, None::<&str>)?,
        ],
    };
    set_menu(app, main, &labels, &items)
}

/// „Ustawienia” with „Język” (a checked item for each choice; `choice` is
/// checked) and „Pamięć podręczna…”.
fn settings_menu(
    app: &AppHandle,
    labels: &MenuLabels,
    choice: LanguageChoice,
) -> tauri::Result<Submenu<Wry>> {
    let choices = LANGUAGE_IDS
        .iter()
        .map(|(id, item)| {
            let text = match item {
                LanguageChoice::System => labels.system_language,
                LanguageChoice::Fixed(language) => own_name(*language),
            };
            CheckMenuItem::with_id(app, *id, text, true, *item == choice, None::<&str>)
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let language = Submenu::with_items(app, labels.language, true, &as_items(&choices))?;
    let cache = MenuItem::with_id(app, CACHE_SETTINGS_ID, labels.cache, true, None::<&str>)?;
    Submenu::with_items(app, labels.settings, true, &[&language, &cache])
}

fn as_items<T: IsMenuItem<Wry>>(items: &[T]) -> Vec<&dyn IsMenuItem<Wry>> {
    items
        .iter()
        .map(|item| item as &dyn IsMenuItem<Wry>)
        .collect()
}

#[cfg(target_os = "macos")]
fn set_menu(
    app: &AppHandle,
    _main: &WebviewWindow,
    _labels: &MenuLabels,
    items: &SitoItems,
) -> tauri::Result<()> {
    let menu = Menu::default(app)?;
    // The app menu comes first: About, a separator, then Sito's settings.
    if let Some(app_menu) = menu
        .items()?
        .first()
        .and_then(|item| item.as_submenu().cloned())
    {
        app_menu.insert(&items.settings, 2)?;
        app_menu.insert(&tauri::menu::PredefinedMenuItem::separator(app)?, 3)?;
    }
    if let Some(help) = menu
        .get(tauri::menu::HELP_SUBMENU_ID)
        .and_then(|item| item.as_submenu().cloned())
    {
        for item in &items.help {
            help.append(item)?;
        }
    }
    app.set_menu(menu)?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn set_menu(
    app: &AppHandle,
    main: &WebviewWindow,
    labels: &MenuLabels,
    items: &SitoItems,
) -> tauri::Result<()> {
    let help = Submenu::with_items(app, labels.help, true, &as_items(&items.help))?;
    main.set_menu(Menu::with_items(app, &[&items.settings, &help])?)?;
    Ok(())
}

/// Runs the action of a clicked menu item.
pub fn on_menu_event(app: &AppHandle, event: &MenuEvent) {
    let Some(action) = MenuAction::of(event.id().as_ref()) else {
        return;
    };
    let event = match action {
        MenuAction::ShowLicenses => SHOW_LICENSES_EVENT,
        MenuAction::ShowCacheSettings => SHOW_CACHE_SETTINGS_EVENT,
        MenuAction::CheckForUpdates => CHECK_REQUESTED_EVENT,
        MenuAction::SetLanguage(choice) => {
            let app = app.clone();
            // Saving touches the database; the menu event runs on the main thread.
            tauri::async_runtime::spawn_blocking(move || change_language(&app, choice));
            return;
        }
    };
    if let Err(err) = app.emit_to(MAIN_LABEL, event, ()) {
        log::warn!("cannot send {event} to the main window: {err}");
    }
}

/// Rebuilds the menu, in the language now in effect and with its choice
/// checked.
pub fn reinstall_menu(app: &AppHandle) {
    let Some(main) = app.get_webview_window(MAIN_LABEL) else {
        return;
    };
    if let Err(err) = install_menu(app, &main) {
        log::warn!("cannot rebuild the menu: {err}");
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
    fn cache_item_shows_the_cache_settings() {
        assert_eq!(
            MenuAction::of(CACHE_SETTINGS_ID),
            Some(MenuAction::ShowCacheSettings)
        );
    }

    #[test]
    fn check_for_updates_item_checks_for_updates() {
        assert_eq!(
            MenuAction::of(CHECK_FOR_UPDATES_ID),
            Some(MenuAction::CheckForUpdates)
        );
    }

    #[test]
    fn language_items_set_their_language() {
        assert_eq!(
            MenuAction::of("language-system"),
            Some(MenuAction::SetLanguage(LanguageChoice::System))
        );
        assert_eq!(
            MenuAction::of("language-pl"),
            Some(MenuAction::SetLanguage(LanguageChoice::Fixed(
                Language::Polish
            )))
        );
        assert_eq!(
            MenuAction::of("language-en"),
            Some(MenuAction::SetLanguage(LanguageChoice::Fixed(
                Language::English
            )))
        );
        assert_eq!(
            MenuAction::of("language-de"),
            Some(MenuAction::SetLanguage(LanguageChoice::Fixed(
                Language::German
            )))
        );
    }

    #[test]
    fn every_choice_has_one_item() {
        let ids: std::collections::HashSet<_> = LANGUAGE_IDS.iter().map(|(id, _)| *id).collect();
        let choices: std::collections::HashSet<_> =
            LANGUAGE_IDS.iter().map(|(_, choice)| *choice).collect();
        assert_eq!(ids.len(), LANGUAGE_IDS.len());
        assert_eq!(choices.len(), LANGUAGE_IDS.len());
    }

    #[test]
    fn system_items_have_no_action() {
        assert_eq!(MenuAction::of("quit"), None);
        assert_eq!(MenuAction::of(""), None);
        assert_eq!(MenuAction::of("language-fr"), None);
    }
}
