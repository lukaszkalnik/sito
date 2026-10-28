import { useEffect, useState } from 'react';

import { onTimelineChanged } from '../reject/api';
import { getSeries, getTimeline, type Series, type TimelineEntry } from './api';

export type TimelineEntriesState =
  | { status: 'loading' }
  | { status: 'ready'; entries: TimelineEntry[]; series: Series[] }
  | { status: 'error' };

/** A result loaded for one scan; `null` means loading it failed. */
type Loaded<T> = { scanKey: unknown; value: T | null };

/**
 * Counts the `timeline-changed` events (photos rejected or restored), so
 * effects that depend on it reload.
 */
function useTimelineRevision(): number {
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    const unlisten = onTimelineChanged(() => setRevision((r) => r + 1));
    return () => void unlisten.then((stop) => stop());
  }, []);
  return revision;
}

/**
 * Loads the timeline and series of the last scan. `scanKey` changes with
 * every finished scan, which reloads both; a new `gap` (seconds) reloads only
 * the series; rejecting or restoring a photo reloads both. While new data
 * loads, the previous data of the same scan stays, so neither dragging the
 * gap slider nor rejecting flashes a loading state.
 */
export function useTimelineEntries(scanKey: unknown, gap: number): TimelineEntriesState {
  const [entries, setEntries] = useState<Loaded<TimelineEntry[]> | null>(null);
  const [series, setSeries] = useState<Loaded<Series[]> | null>(null);
  const revision = useTimelineRevision();

  useEffect(() => {
    let cancelled = false;
    getTimeline().then(
      (value) => !cancelled && setEntries({ scanKey, value }),
      () => !cancelled && setEntries({ scanKey, value: null }),
    );
    return () => {
      cancelled = true;
    };
  }, [scanKey, revision]);

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
  }, [scanKey, gap, revision]);

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
