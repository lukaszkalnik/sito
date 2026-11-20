import { useTranslation } from 'react-i18next';

/** Number and date formats of one locale. */
export interface Formats {
  /** The locale (BCP 47) the formats are for. */
  readonly locale: string;
  /** A day with its weekday, e.g. „sobota, 1 czerwca 2024”, “Saturday, 1 June 2024”. */
  readonly longDay: (date: Date) => string;
  /** A fraction as a whole percentage, e.g. `0.5` → „50%”. */
  readonly percent: (fraction: number) => string;
  /** A number rounded to a whole number, grouped as the locale groups it. */
  readonly whole: (value: number) => string;
  /** A number with exactly one decimal, e.g. „7,5”, “7.5”. */
  readonly oneDecimal: (value: number) => string;
}

/** The locale of each UI language whose locale gives no region. */
const DEFAULT_LOCALES: Readonly<Record<string, string>> = {
  pl: 'pl-PL',
  en: 'en-GB',
  de: 'de-DE',
};

const cache = new Map<string, Formats>();

/** The formats of `locale` (a BCP 47 tag), built once per locale. */
export function formatsFor(locale: string): Formats {
  const tag = DEFAULT_LOCALES[locale] ?? locale;
  const cached = cache.get(tag);
  if (cached) return cached;
  const formats = buildFormats(tag);
  cache.set(tag, formats);
  return formats;
}

function buildFormats(locale: string): Formats {
  const longDay = new Intl.DateTimeFormat(locale, {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  });
  const percent = new Intl.NumberFormat(locale, { style: 'percent', maximumFractionDigits: 0 });
  const whole = new Intl.NumberFormat(locale, { maximumFractionDigits: 0 });
  const oneDecimal = new Intl.NumberFormat(locale, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  });
  return {
    locale,
    longDay: (date) => longDay.format(date),
    percent: (fraction) => percent.format(fraction),
    whole: (value) => whole.format(value),
    oneDecimal: (value) => oneDecimal.format(value),
  };
}

/** The formats of the UI's current language; re-renders when it changes. */
export function useFormats(): Formats {
  const { i18n } = useTranslation();
  return formatsFor(i18n.language);
}
