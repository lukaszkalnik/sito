import { useEffect, useState } from 'react';

import { useTimelineRevision } from '../reject/useTimelineRevision';
import { getAcceptedIds, onAcceptedChanged } from './api';

const NONE: ReadonlySet<number> = new Set();

/** Counts the `accepted-changed` events, so effects that depend on it reload. */
function useAcceptedRevision(): number {
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    const unlisten = onAcceptedChanged(() => setRevision((r) => r + 1));
    return () => void unlisten.then((stop) => stop());
  }, []);
  return revision;
}

/**
 * The ids of the accepted photos of scan `scanId`, reloaded whenever any
 * window accepts a photo, takes a mark off, rejects or restores one.
 */
export function useAcceptedIds(scanId: number): ReadonlySet<number> {
  const timelineRevision = useTimelineRevision();
  const acceptedRevision = useAcceptedRevision();
  // Remembers which scan the ids belong to, so another scan never shows them.
  const [loaded, setLoaded] = useState<{ scanId: number; ids: ReadonlySet<number> } | null>(null);
  useEffect(() => {
    let active = true;
    getAcceptedIds().then(
      (ids) => active && setLoaded({ scanId, ids: new Set(ids) }),
      (err: unknown) => console.error('Cannot load the accepted photos', err),
    );
    return () => {
      active = false;
    };
  }, [scanId, timelineRevision, acceptedRevision]);
  return loaded?.scanId === scanId ? loaded.ids : NONE;
}
