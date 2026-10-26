const dayFormat = new Intl.DateTimeFormat('pl-PL', {
  weekday: 'long',
  day: 'numeric',
  month: 'long',
  year: 'numeric',
});

/** Formats a calendar day (`YYYY-MM-DD`) in Polish, e.g. `sobota, 1 czerwca 2024`. */
export function formatDay(date: string): string {
  const [year, month, day] = date.split('-').map(Number);
  return dayFormat.format(new Date(year ?? 1970, (month ?? 1) - 1, day ?? 1));
}

/** The time of day (`HH:mm:ss`) of a local ISO timestamp. */
export function timeOf(takenAt: string): string {
  return takenAt.slice(11, 19);
}
