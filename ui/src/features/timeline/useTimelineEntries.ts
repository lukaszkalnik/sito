import { useEffect, useState } from 'react';

import { getSeries, getTimeline, type Series, type TimelineEntry } from '../../api/scan';

export type TimelineEntriesState =
  | { status: 'loading' }
  | { status: 'ready'; entries: TimelineEntry[]; series: Series[] }
  | { status: 'error' };

/** A result loaded for one scan; `null` means loading it failed. */
type Loaded<T> = { scanKey: unknown; value: T | null };

/**
 * Loads the timeline and series of the last scan. `scanKey` changes with
 * every finished scan, which reloads both; a new `gap` (seconds) reloads only
 * the series. While new series load, the previous ones of the same scan stay,
 * so dragging the gap slider does not flash a loading state.
 */
export function useTimelineEntries(scanKey: unknown, gap: number): TimelineEntriesState {
  const [entries, setEntries] = useState<Loaded<TimelineEntry[]> | null>(null);
  const [series, setSeries] = useState<Loaded<Series[]> | null>(null);

  useEffect(() => {
    let cancelled = false;
    getTimeline().then(
      (value) => !cancelled && setEntries({ scanKey, value }),
      () => !cancelled && setEntries({ scanKey, value: null }),
    );
    return () => {
      cancelled = true;
    };
  }, [scanKey]);

  useEffect(() => {
    // Only the latest request counts; slower answers for older gaps are dropped.
    let cancelled = false;
    getSeries(gap).then(
      (value) => !cancelled && setSeries({ scanKey, value }),
      () => !cancelled && setSeries({ scanKey, value: null }),
    );
    return () => {
      cancelled = true;
    };
  }, [scanKey, gap]);

  const current = (loaded: Loaded<unknown> | null) => loaded !== null && loaded.scanKey === scanKey;
  if (
    (current(entries) && entries?.value === null) ||
    (current(series) && series?.value === null)
  ) {
    return { status: 'error' };
  }
  if (current(entries) && current(series) && entries?.value && series?.value) {
    return { status: 'ready', entries: entries.value, series: series.value };
  }
  return { status: 'loading' };
}
