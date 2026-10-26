import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

import pl from './pl.json';

export const defaultLanguage = 'pl';

// Plural suffixes (_one, _few, _many, _other) are resolved by i18next
// via Intl.PluralRules, so Polish plural forms work out of the box.
void i18n.use(initReactI18next).init({
  lng: defaultLanguage,
  fallbackLng: defaultLanguage,
  resources: {
    pl: { translation: pl },
  },
  interpolation: {
    // React already escapes rendered values.
    escapeValue: false,
  },
});

export default i18n;
