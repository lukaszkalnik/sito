import { useEffect, useState } from 'react';

import { getTimeline, type TimelineEntry } from '../../api/scan';

export type TimelineEntriesState =
  { status: 'loading' } | { status: 'ready'; entries: TimelineEntry[] } | { status: 'error' };

type Loaded = { scanId: unknown; state: Exclude<TimelineEntriesState, { status: 'loading' }> };

/**
 * Loads the timeline of the last scan. `scanId` changes with every finished
 * scan, which triggers a reload; until it arrives the state is `loading`.
 */
export function useTimelineEntries(scanId: unknown): TimelineEntriesState {
  const [loaded, setLoaded] = useState<Loaded | null>(null);

  useEffect(() => {
    let cancelled = false;
    getTimeline().then(
      (entries) => {
        if (!cancelled) setLoaded({ scanId, state: { status: 'ready', entries } });
      },
      () => {
        if (!cancelled) setLoaded({ scanId, state: { status: 'error' } });
      },
    );
    return () => {
      cancelled = true;
    };
  }, [scanId]);

  return loaded !== null && loaded.scanId === scanId ? loaded.state : { status: 'loading' };
}
