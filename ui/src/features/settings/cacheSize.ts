import type { Formats } from '../../i18n/formats';

/** Bytes in a kilobyte and a megabyte, binary as Windows Explorer counts them. */
const KB = 1024;
const MB = KB * KB;

/** A size in the cache settings: whole MB from 1 MB („412”), below in whole kB (also 0). */
export interface CacheSizeText {
  readonly value: string;
  readonly unit: 'mb' | 'kb';
}

/** `bytes` as a {@link CacheSizeText} in the locale's number format. */
export function cacheSizeText(bytes: number, formats: Formats): CacheSizeText {
  if (bytes >= MB) return { value: formats.whole(bytes / MB), unit: 'mb' };
  return { value: formats.whole(bytes / KB), unit: 'kb' };
}
