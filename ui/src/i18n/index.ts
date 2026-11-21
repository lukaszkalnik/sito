import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

import { LANGUAGES } from './api';
import de from './de.json';
import en from './en.json';
import pl from './pl.json';

// The language is a locale such as `de-AT`, so i18next's plural rules and
// number formats follow it; its texts come from the language's resource
// (`de`). Plural suffixes (_one, _few, _many, _other) are resolved via
// Intl.PluralRules.
void i18n.use(initReactI18next).init({
  // Polish until the backend says otherwise (see `startLanguage`); tests stay in Polish.
  lng: 'pl',
  fallbackLng: 'en',
  supportedLngs: LANGUAGES,
  nonExplicitSupportedLngs: true,
  resources: {
    pl: { translation: pl },
    en: { translation: en },
    de: { translation: de },
  },
  interpolation: {
    // React already escapes rendered values.
    escapeValue: false,
  },
  // Every key is in every language (checked by a test); a missing one is a bug.
  saveMissing: true,
  missingKeyHandler: (languages, _namespace, key) => {
    console.error(`missing translation: ${key} (${languages.join(', ')})`);
  },
});

export default i18n;
