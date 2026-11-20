//! The locale for dates and numbers: the system locale when it has the UI's
//! language (an American gets American dates in English), else the
//! language's own default.

use super::Language;

/// The locale for dates and numbers in `language`, given the system locale
/// `system` (a BCP 47 tag, or a POSIX one such as `de_AT.UTF-8`).
pub fn format_locale(language: Language, system: Option<&str>) -> String {
    system
        .and_then(normalized)
        .filter(|tag| language_subtag(tag).is_some_and(|code| code == language.code()))
        .unwrap_or_else(|| default_locale(language).to_owned())
}

/// The locale of `language` when the system's has another language.
const fn default_locale(language: Language) -> &'static str {
    match language {
        Language::Polish => "pl-PL",
        Language::English => "en-GB",
        Language::German => "de-DE",
    }
}

/// The lowercase language subtag of `locale` (`de` of `de-AT` or
/// `de_AT.UTF-8`), if it has a well-formed one.
pub fn language_subtag(locale: &str) -> Option<String> {
    let tag = normalized(locale)?;
    let first = tag.split('-').next()?;
    let letters = (2..=3).contains(&first.len()) && first.bytes().all(|b| b.is_ascii_alphabetic());
    letters.then(|| first.to_ascii_lowercase())
}

/// `locale` as a BCP 47 tag: the encoding and modifier of a POSIX locale cut
/// off, `_` turned into `-`. `None` if it is empty or has characters a tag
/// cannot have, so the UI never gets text that is not a locale.
fn normalized(locale: &str) -> Option<String> {
    let base = locale.split(['.', '@']).next()?.replace('_', "-");
    let well_formed = base.split('-').all(|part| {
        (1..=8).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_alphanumeric())
    });
    well_formed.then_some(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_locale_of_the_same_language_is_used() {
        assert_eq!(format_locale(Language::English, Some("en-US")), "en-US");
        assert_eq!(format_locale(Language::German, Some("de-AT")), "de-AT");
        assert_eq!(format_locale(Language::German, Some("de-CH")), "de-CH");
        assert_eq!(format_locale(Language::Polish, Some("pl-PL")), "pl-PL");
    }

    #[test]
    fn another_system_language_gets_the_default_locale() {
        assert_eq!(format_locale(Language::English, Some("pl-PL")), "en-GB");
        assert_eq!(format_locale(Language::English, Some("fr-FR")), "en-GB");
        assert_eq!(format_locale(Language::German, Some("en-US")), "de-DE");
        assert_eq!(format_locale(Language::Polish, Some("de-DE")), "pl-PL");
    }

    #[test]
    fn no_or_a_broken_system_locale_gets_the_default_locale() {
        assert_eq!(format_locale(Language::Polish, None), "pl-PL");
        assert_eq!(format_locale(Language::English, Some("")), "en-GB");
        assert_eq!(format_locale(Language::English, Some("en US")), "en-GB");
        assert_eq!(format_locale(Language::English, Some("en--US")), "en-GB");
        assert_eq!(format_locale(Language::English, Some("C")), "en-GB");
    }

    #[test]
    fn posix_locales_become_tags() {
        assert_eq!(
            format_locale(Language::German, Some("de_AT.UTF-8")),
            "de-AT"
        );
        assert_eq!(format_locale(Language::German, Some("de_DE@euro")), "de-DE");
    }

    #[test]
    fn a_language_alone_is_a_locale() {
        assert_eq!(format_locale(Language::German, Some("de")), "de");
    }

    #[test]
    fn scripts_and_extensions_are_kept() {
        assert_eq!(
            format_locale(Language::English, Some("en-Latn-US")),
            "en-Latn-US"
        );
    }

    #[test]
    fn language_subtags_ignore_letter_case() {
        assert_eq!(language_subtag("DE-at").as_deref(), Some("de"));
        assert_eq!(language_subtag("12-AT"), None);
        assert_eq!(language_subtag("e-US"), None);
    }
}
