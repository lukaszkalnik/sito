import type { Formats } from '../../i18n/formats';

/** Formats a calendar day (`YYYY-MM-DD`) with its weekday, e.g. `sobota, 1 czerwca 2024`. */
export function formatDay(date: string, formats: Formats): string {
  const [year, month, day] = date.split('-').map(Number);
  return formats.longDay(new Date(year ?? 1970, (month ?? 1) - 1, day ?? 1));
}

/** The time of day (`HH:mm:ss`) of a local ISO timestamp. */
export function timeOf(takenAt: string): string {
  return takenAt.slice(11, 19);
}
