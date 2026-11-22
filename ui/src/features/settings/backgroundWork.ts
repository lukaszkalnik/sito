/**
 * What the open folder's background work is doing, as far as the cache
 * settings care: a scan or refresh (`scanning`) keeps the cache from moving
 * and its photo data from being emptied; the EXIF pass (`exifPass`) only
 * the latter.
 */
export type BackgroundWork = 'none' | 'exifPass' | 'scanning';
