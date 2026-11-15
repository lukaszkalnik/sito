import { cpSync, mkdirSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const FIXTURES = fileURLToPath(new URL('../../fixtures/', import.meta.url));

/**
 * The photos of the test gallery, from `fixtures/README.md`. In the „Serie”
 * view (one camera's photos at most 10 s apart form a series; a series shows
 * at its newest photo, newest first) they come in this order.
 */
export const SERIES_ORDER = [
  // Canon EOS R6, 12:00:06 and 12:00:02: one series.
  'canon_12-00-06.jpg',
  'canon_12-00-02.jpg',
  // Google Pixel 8, 12:00:04 and 12:00:00: the other one.
  'pixel_12-00-04.jpg',
  'pixel_12-00-00.jpg',
] as const;

/** Photos outside any series: another camera's single shot, and one without EXIF. */
const SINGLES = ['offset_2024-06-02_08-30-00+0200.jpg', 'no-exif.jpg'] as const;

/** What the scan of the gallery finds. */
export const EXPECTED = { photos: SERIES_ORDER.length + SINGLES.length, series: 2 } as const;

/** A gallery of copies of `fixtures/`, in a new temp folder. */
export interface Gallery {
  /** The folder the app opens; its name has Polish letters. */
  readonly dir: string;
  /** The path of `name` in the gallery. */
  readonly path: (name: string) => string;
  /** The path of `name` in the gallery's `Odrzucone` folder. */
  readonly rejectedPath: (name: string) => string;
}

/** Builds a new gallery; modification times are kept, as the scan reads them. */
export function makeGallery(): Gallery {
  const dir = join(mkdtempSync(join(tmpdir(), 'sito-e2e-')), 'Zdjęcia – próba');
  mkdirSync(dir);
  const copy = (folder: string, name: string) =>
    cpSync(join(FIXTURES, folder, name), join(dir, name), { preserveTimestamps: true });
  SERIES_ORDER.forEach((name) => copy('cameras', name));
  SINGLES.forEach((name) => copy('exif', name));
  return {
    dir,
    path: (name) => join(dir, name),
    rejectedPath: (name) => join(dir, 'Odrzucone', name),
  };
}
