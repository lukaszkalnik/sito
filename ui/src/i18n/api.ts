import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** A language of the UI. Mirrors `Language` in `src-tauri/src/language.rs`. */
export type Language = 'pl' | 'en' | 'de';

/** What the user chose: the system language or a fixed one. Mirrors `LanguageChoice`. */
export type LanguageChoice = 'system' | Language;

/** The language in effect. Mirrors `LanguageDto`. */
export interface LanguageInfo {
  readonly language: Language;
  /** BCP 47 locale for dates and numbers; always of `language`. */
  readonly locale: string;
  readonly choice: LanguageChoice;
}

/** Every language of the UI, the first one being the app's own. */
export const LANGUAGES: readonly Language[] = ['pl', 'en', 'de'];

/** Mirrors `LANGUAGE_CHANGED`. */
const LANGUAGE_CHANGED = 'language-changed';

const isLanguage = (value: unknown): value is Language => LANGUAGES.includes(value as Language);

/** Whether `locale` is a well-formed BCP 47 tag of `language` that `Intl` accepts. */
function isLocaleOf(locale: unknown, language: Language): locale is string {
  if (typeof locale !== 'string') return false;
  try {
    const [canonical] = Intl.getCanonicalLocales(locale);
    return canonical?.split('-')[0]?.toLowerCase() === language;
  } catch {
    return false;
  }
}

/**
 * `raw` as a {@link LanguageInfo}, or `null` if it is not one. A locale that is
 * not of the language becomes the language alone, which `Intl` always accepts.
 */
export function toLanguageInfo(raw: unknown): LanguageInfo | null {
  if (typeof raw !== 'object' || raw === null) return null;
  const info = raw as Partial<Record<keyof LanguageInfo, unknown>>;
  const { language, locale, choice } = info;
  if (!isLanguage(language)) return null;
  if (choice !== 'system' && !isLanguage(choice)) return null;
  return { language, locale: isLocaleOf(locale, language) ? locale : language, choice };
}

/** The language in effect, or `null` (logged) if the backend's answer is not one. */
export async function getLanguage(): Promise<LanguageInfo | null> {
  const raw = await invoke<unknown>('get_language');
  const info = toLanguageInfo(raw);
  if (info === null) console.error('get_language returned an invalid language', raw);
  return info;
}

/** Calls `handler` with the new language whenever it changes, in any window. */
export async function onLanguageChanged(
  handler: (info: LanguageInfo) => void,
): Promise<UnlistenFn> {
  return listen<unknown>(LANGUAGE_CHANGED, (event) => {
    const info = toLanguageInfo(event.payload);
    if (info === null) console.error('language-changed sent an invalid language', event.payload);
    else handler(info);
  });
}
