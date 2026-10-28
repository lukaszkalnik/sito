import type { PhotoFormat } from './api';

/** Translation keys of the badge naming a photo's file format; `null` for none. */
export const FORMAT_BADGES: Readonly<Record<PhotoFormat, string | null>> = {
  jpeg: null,
  raw: 'format.raw',
  rawJpeg: 'format.rawJpeg',
  tiff: 'format.tiff',
};
