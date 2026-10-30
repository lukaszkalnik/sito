import { useEffect, useRef, useState } from 'react';

import { useTimelineRevision } from '../reject/useTimelineRevision';
import { getSeries, getTimeline, type Series, type TimelineEntry } from './api';
import { arrangeEntries } from './arrange';

export type TimelineEntriesState =
  | { status: 'loading' }
  /** `entries` are in display order; `series` are ranges of it. */
  | { status: 'ready'; entries: TimelineEntry[]; series: readonly Series[] }
  | { status: 'error' };

/** The arranged timeline loaded for one scan; `null` means loading it failed. */
type Loaded = {
  scanKey: unknown;
  value: { entries: TimelineEntry[]; series: readonly Series[] } | null;
};

/** Timeline entries in timeline order, with what they were loaded for. */
type Fetched = { scanKey: unknown; revision: number; entries: TimelineEntry[] };

/**
 * Loads the timeline and series of the last scan and arranges the entries in
 * display order. `scanKey` changes with every finished scan, which reloads
 * both; a new `gap` (seconds) reloads only the series; rejecting or restoring
 * a photo reloads both. Entries and series are always set together, so they
 * match. While new data loads, the previous data of the same scan stays, so
 * neither dragging the gap slider nor rejecting flashes a loading state.
 */
export function useTimelineEntries(scanKey: unknown, gap: number): TimelineEntriesState {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const fetched = useRef<Fetched | null>(null);
  const revision = useTimelineRevision();

  useEffect(() => {
    // Only the latest request counts; slower answers for older gaps are dropped.
    let cancelled = false;
    const known = fetched.current;
    const timeline =
      known !== null && known.scanKey === scanKey && known.revision === revision
        ? Promise.resolve(known.entries)
        : getTimeline();
    Promise.all([timeline, getSeries(gap)]).then(
      ([entries, grouping]) => {
        if (cancelled) return;
        fetched.current = { scanKey, revision, entries };
        const arranged = arrangeEntries(entries, grouping.order);
        if (arranged === null) {
          // The timeline changed between the two answers; the change event
          // that follows loads both again.
          console.warn('Series do not match the timeline; waiting for the next change.');
          return;
        }
        setLoaded({ scanKey, value: { entries: arranged, series: grouping.series } });
      },
      () => !cancelled && setLoaded({ scanKey, value: null }),
    );
    return () => {
      cancelled = true;
    };
  }, [scanKey, gap, revision]);

  if (loaded === null || loaded.scanKey !== scanKey) return { status: 'loading' };
  if (loaded.value === null) return { status: 'error' };
  return { status: 'ready', ...loaded.value };
}
