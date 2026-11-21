import i18n from './index';
import { getLanguage, onLanguageChanged, type LanguageInfo } from './api';

/** Shows the UI in `info`'s language, with its locale for dates, numbers and `<html lang>`. */
export async function applyLanguage(info: LanguageInfo): Promise<void> {
  await i18n.changeLanguage(info.locale);
  document.documentElement.lang = info.locale;
}

/**
 * Loads the language from the backend and follows its changes. Await it
 * before the first render, so nothing shows in another language first. If
 * the backend cannot say, the UI stays Polish (logged).
 */
export async function startLanguage(): Promise<void> {
  try {
    const info = await getLanguage();
    if (info !== null) await applyLanguage(info);
  } catch (error) {
    console.error('cannot read the language', error);
  }
  onLanguageChanged((info) => void applyLanguage(info)).catch((error: unknown) => {
    console.error('cannot follow language changes', error);
  });
}
