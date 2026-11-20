//! The UI language: Polish, English or German, following the system
//! language unless the user chose one. The choice is kept in the action
//! log's settings, since the backend needs the language before any window
//! loads (the menu) and for every rejection (the folder name).

mod locale;

use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use sito_core::actions::ActionLog;
use tauri::{AppHandle, Manager};

use crate::error::{CommandError, off_main_thread};
use crate::reject::{Actions, announce_with};

/// Event sent to all windows when the language changes, with a
/// [`LanguageDto`].
pub const LANGUAGE_CHANGED: &str = "language-changed";
/// Key of the user's choice in the settings.
const SETTING_KEY: &str = "language";

/// A language of the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Language {
    /// Polish, the first language of the app.
    #[serde(rename = "pl")]
    Polish,
    /// English, also for every system language Sito has no translation of.
    #[serde(rename = "en")]
    English,
    /// German.
    #[serde(rename = "de")]
    German,
}

impl Language {
    /// The language's ISO 639-1 code, as in the names of the translation
    /// files.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Polish => "pl",
            Self::English => "en",
            Self::German => "de",
        }
    }

    /// The language with the ISO 639-1 `code` (any letter case), if the UI
    /// has it.
    fn of_code(code: &str) -> Option<Self> {
        [Self::Polish, Self::English, Self::German]
            .into_iter()
            .find(|language| language.code().eq_ignore_ascii_case(code))
    }

    /// The language of the system locale `locale` (a BCP 47 tag such as
    /// `de-AT`), or English if the UI has no translation of it or the
    /// system reports none.
    pub fn of_system(locale: Option<&str>) -> Self {
        locale
            .and_then(locale::language_subtag)
            .and_then(|code| Self::of_code(&code))
            .unwrap_or(Self::English)
    }
}

/// The language the user chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum LanguageChoice {
    /// The system language (the default).
    #[default]
    System,
    /// This language, whatever the system's.
    Fixed(Language),
}

/// Text that is not a [`LanguageChoice`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownChoice(String);

impl std::fmt::Display for UnknownChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown language choice {:?}", self.0)
    }
}

impl TryFrom<String> for LanguageChoice {
    type Error = UnknownChoice;

    /// `system`, or the code of a language (`pl`, `en`, `de`).
    fn try_from(text: String) -> Result<Self, Self::Error> {
        if text == "system" {
            return Ok(Self::System);
        }
        Language::of_code(&text)
            .filter(|language| language.code() == text)
            .map(Self::Fixed)
            .ok_or(UnknownChoice(text))
    }
}

impl From<LanguageChoice> for String {
    fn from(choice: LanguageChoice) -> Self {
        match choice {
            LanguageChoice::System => "system".to_owned(),
            LanguageChoice::Fixed(language) => language.code().to_owned(),
        }
    }
}

/// The language in effect, sent to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageDto {
    /// The language of the texts.
    pub language: Language,
    /// The BCP 47 locale for dates and numbers.
    pub locale: String,
    /// What the user chose.
    pub choice: LanguageChoice,
}

/// The system locale, read once at start, and the user's choice.
#[derive(Debug)]
pub struct LanguageState {
    system_locale: Option<String>,
    choice: Mutex<LanguageChoice>,
}

impl LanguageState {
    /// The state with the system's locale and the choice saved in `log`
    /// (the system language if none is saved or the log is not open).
    pub fn load(log: Option<&ActionLog>) -> Self {
        Self::new(
            sys_locale::get_locale(),
            log.map_or_else(LanguageChoice::default, saved_choice),
        )
    }

    fn new(system_locale: Option<String>, choice: LanguageChoice) -> Self {
        Self {
            system_locale,
            choice: Mutex::new(choice),
        }
    }

    fn choice(&self) -> MutexGuard<'_, LanguageChoice> {
        self.choice.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The language in effect, its locale and the user's choice.
    pub fn current(&self) -> LanguageDto {
        let choice = *self.choice();
        let system = self.system_locale.as_deref();
        let language = match choice {
            LanguageChoice::System => Language::of_system(system),
            LanguageChoice::Fixed(language) => language,
        };
        LanguageDto {
            language,
            locale: locale::format_locale(language, system),
            choice,
        }
    }

    fn set(&self, choice: LanguageChoice) {
        *self.choice() = choice;
    }
}

/// The choice saved in `log`, or the system language (with a warning) if
/// it cannot be read.
fn saved_choice(log: &ActionLog) -> LanguageChoice {
    match log.setting(SETTING_KEY) {
        Ok(None) => LanguageChoice::System,
        Ok(Some(text)) => LanguageChoice::try_from(text).unwrap_or_else(|err| {
            log::warn!("{err}; using the system language");
            LanguageChoice::System
        }),
        Err(err) => {
            log::warn!("cannot read the language: {err}");
            LanguageChoice::System
        }
    }
}

/// The language in effect, its locale for dates and numbers and the user's
/// choice.
// Tauri injects `State` by value into commands.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn get_language(state: tauri::State<'_, LanguageState>) -> LanguageDto {
    state.current()
}

/// Sets the user's choice, saves it and tells every window the language in
/// effect.
#[tauri::command]
pub async fn set_language(
    choice: LanguageChoice,
    app: AppHandle,
) -> Result<LanguageDto, CommandError> {
    off_main_thread(move || change_language(&app, choice)).await
}

/// Applies and saves `choice` and tells every window. A choice that cannot
/// be saved still applies until the app closes.
pub fn change_language(app: &AppHandle, choice: LanguageChoice) -> LanguageDto {
    save_choice(app, choice);
    let state = app.state::<LanguageState>();
    state.set(choice);
    let current = state.current();
    announce_with(app, LANGUAGE_CHANGED, current.clone());
    current
}

fn save_choice(app: &AppHandle, choice: LanguageChoice) {
    let actions = app.state::<Actions>().inner().lock();
    let Some(log) = actions.opened_log() else {
        log::warn!("no action log; the language is not saved");
        return;
    };
    if let Err(err) = log.set_setting(SETTING_KEY, &String::from(choice)) {
        log::warn!("cannot save the language: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_languages_with_a_translation_are_used() {
        assert_eq!(Language::of_system(Some("pl-PL")), Language::Polish);
        assert_eq!(Language::of_system(Some("en-US")), Language::English);
        assert_eq!(Language::of_system(Some("de-AT")), Language::German);
        assert_eq!(Language::of_system(Some("de-CH")), Language::German);
    }

    #[test]
    fn other_system_languages_get_english() {
        assert_eq!(Language::of_system(Some("fr-FR")), Language::English);
        assert_eq!(Language::of_system(Some("zh-Hans-CN")), Language::English);
    }

    #[test]
    fn no_or_empty_system_locale_gets_english() {
        assert_eq!(Language::of_system(None), Language::English);
        assert_eq!(Language::of_system(Some("")), Language::English);
    }

    #[test]
    fn posix_style_system_locales_are_understood() {
        assert_eq!(Language::of_system(Some("pl_PL.UTF-8")), Language::Polish);
        assert_eq!(Language::of_system(Some("DE")), Language::German);
    }

    #[test]
    fn choices_read_back_from_their_text() {
        for choice in [
            LanguageChoice::System,
            LanguageChoice::Fixed(Language::Polish),
            LanguageChoice::Fixed(Language::English),
            LanguageChoice::Fixed(Language::German),
        ] {
            assert_eq!(LanguageChoice::try_from(String::from(choice)), Ok(choice));
        }
    }

    #[test]
    fn unknown_choices_are_refused() {
        for text in ["", "fr", "PL", "System", "pl-PL"] {
            assert!(LanguageChoice::try_from(text.to_owned()).is_err(), "{text}");
        }
    }

    #[test]
    fn the_system_choice_follows_the_system() {
        let state = LanguageState::new(Some("de-AT".to_owned()), LanguageChoice::System);
        assert_eq!(
            state.current(),
            LanguageDto {
                language: Language::German,
                locale: "de-AT".to_owned(),
                choice: LanguageChoice::System,
            }
        );
    }

    #[test]
    fn a_fixed_choice_overrides_the_system() {
        let state = LanguageState::new(
            Some("de-AT".to_owned()),
            LanguageChoice::Fixed(Language::Polish),
        );
        assert_eq!(state.current().language, Language::Polish);
        assert_eq!(state.current().locale, "pl-PL");
    }

    #[test]
    fn setting_a_choice_changes_the_language() {
        let state = LanguageState::new(Some("pl-PL".to_owned()), LanguageChoice::System);
        state.set(LanguageChoice::Fixed(Language::English));
        assert_eq!(state.current().language, Language::English);
        assert_eq!(state.current().locale, "en-GB");
        state.set(LanguageChoice::System);
        assert_eq!(state.current().language, Language::Polish);
    }

    #[test]
    fn a_saved_choice_is_loaded() {
        let log = ActionLog::open_in_memory().unwrap();
        log.set_setting(SETTING_KEY, "de").unwrap();
        assert_eq!(saved_choice(&log), LanguageChoice::Fixed(Language::German));
    }

    #[test]
    fn no_or_a_broken_saved_choice_is_the_system_language() {
        let log = ActionLog::open_in_memory().unwrap();
        assert_eq!(saved_choice(&log), LanguageChoice::System);
        log.set_setting(SETTING_KEY, "klingon").unwrap();
        assert_eq!(saved_choice(&log), LanguageChoice::System);
    }

    #[test]
    fn the_dto_serializes_as_the_ui_expects() {
        let state = LanguageState::new(None, LanguageChoice::Fixed(Language::German));
        assert_eq!(
            serde_json::to_value(state.current()).unwrap(),
            serde_json::json!({ "language": "de", "locale": "de-DE", "choice": "de" })
        );
    }

    #[test]
    fn choices_deserialize_from_the_ui() {
        let choice: LanguageChoice = serde_json::from_str("\"system\"").unwrap();
        assert_eq!(choice, LanguageChoice::System);
        assert!(serde_json::from_str::<LanguageChoice>("\"fr\"").is_err());
    }
}
