import type { Formats } from '../../i18n/formats';

/** Bytes in a kilobyte and a megabyte, binary as Windows Explorer counts them. */
const KB = 1024;
const MB = KB * KB;

/** A file size and its unit: from 1 MB in MB with one decimal („7,5”), below in whole kB. */
export interface FileSize {
  readonly value: string;
  readonly unit: 'mb' | 'kb';
}

/** `bytes` as a {@link FileSize}; anything above zero shows at least 1 kB. */
export function fileSize(bytes: number, formats: Formats): FileSize {
  if (bytes >= MB) return { value: formats.oneDecimal(bytes / MB), unit: 'mb' };
  return { value: formats.whole(Math.max(1, Math.round(bytes / KB))), unit: 'kb' };
}
