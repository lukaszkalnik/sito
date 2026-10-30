/** Bytes in a kilobyte and a megabyte, binary as Windows Explorer counts them. */
const KB = 1024;
const MB = KB * KB;

const megabytes = new Intl.NumberFormat('pl-PL', {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
});
const whole = new Intl.NumberFormat('pl-PL', { maximumFractionDigits: 0 });

/** A file size and its unit: from 1 MB in MB with one decimal („7,5”), below in whole kB. */
export interface FileSize {
  readonly value: string;
  readonly unit: 'mb' | 'kb';
}

/** `bytes` as a {@link FileSize}; anything above zero shows at least 1 kB. */
export function fileSize(bytes: number): FileSize {
  if (bytes >= MB) return { value: megabytes.format(bytes / MB), unit: 'mb' };
  return { value: whole.format(Math.max(1, Math.round(bytes / KB))), unit: 'kb' };
}

/** A pixel count in `pl-PL` („5472”, „10 000”). */
export function formatPixels(pixels: number): string {
  return whole.format(pixels);
}
