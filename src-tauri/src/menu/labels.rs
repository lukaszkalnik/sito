//! The labels of Sito's native menu items in each UI language. A native
//! menu cannot use the webview's translation files, so they live here.

use crate::language::Language;

/// The labels of the menu items Sito adds, in one language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuLabels {
    /// The Help menu (Windows; macOS has its own).
    pub help: &'static str,
    /// Shows the third-party notices.
    pub licenses: &'static str,
    /// Checks for a newer version.
    pub check_for_updates: &'static str,
    /// The settings menu.
    pub settings: &'static str,
    /// The language submenu of the settings.
    pub language: &'static str,
    /// The choice to follow the system language.
    pub system_language: &'static str,
    /// Shows the cache settings.
    pub cache: &'static str,
}

impl MenuLabels {
    /// The labels in `language`.
    pub const fn of(language: Language) -> Self {
        match language {
            Language::Polish => Self {
                help: "Pomoc",
                licenses: "Licencje…",
                check_for_updates: "Sprawdź aktualizacje…",
                settings: "Ustawienia",
                language: "Język",
                system_language: "Język systemu",
                cache: "Pamięć podręczna…",
            },
            Language::English => Self {
                help: "Help",
                licenses: "Licenses…",
                check_for_updates: "Check for Updates…",
                settings: "Settings",
                language: "Language",
                system_language: "System Language",
                cache: "Cache…",
            },
            Language::German => Self {
                help: "Hilfe",
                licenses: "Lizenzen…",
                check_for_updates: "Nach Updates suchen…",
                settings: "Einstellungen",
                language: "Sprache",
                system_language: "Systemsprache",
                cache: "Zwischenspeicher…",
            },
        }
    }
}

/// The name of `language` in itself, as the language menu shows it in
/// every language, so a user who cannot read the current one finds theirs.
pub const fn own_name(language: Language) -> &'static str {
    match language {
        Language::Polish => "Polski",
        Language::English => "English",
        Language::German => "Deutsch",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LANGUAGES: [Language; 3] = [Language::Polish, Language::English, Language::German];

    fn all(labels: MenuLabels) -> [&'static str; 7] {
        [
            labels.cache,
            labels.help,
            labels.licenses,
            labels.check_for_updates,
            labels.settings,
            labels.language,
            labels.system_language,
        ]
    }

    #[test]
    fn every_label_exists_in_every_language() {
        for language in LANGUAGES {
            for label in all(MenuLabels::of(language)) {
                assert!(!label.trim().is_empty(), "{language:?}");
            }
        }
    }

    #[test]
    fn labels_of_one_language_differ() {
        for language in LANGUAGES {
            let labels = all(MenuLabels::of(language));
            for (i, label) in labels.iter().enumerate() {
                assert!(!labels[i + 1..].contains(label), "{language:?}: {label}");
            }
        }
    }

    #[test]
    fn items_that_open_a_window_end_with_an_ellipsis() {
        for language in LANGUAGES {
            let labels = MenuLabels::of(language);
            assert!(labels.licenses.ends_with('…'), "{language:?}");
            assert!(labels.cache.ends_with('…'), "{language:?}");
            assert!(labels.check_for_updates.ends_with('…'), "{language:?}");
        }
    }

    #[test]
    fn languages_are_named_in_themselves() {
        assert_eq!(LANGUAGES.map(own_name), ["Polski", "English", "Deutsch"]);
    }
}
