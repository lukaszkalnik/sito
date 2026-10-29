import { useEffect, useState } from 'react';

import { onTimelineChanged } from './api';

/**
 * Counts the `timeline-changed` events (photos rejected or restored, the
 * timeline refreshed), so effects that depend on it reload.
 */
export function useTimelineRevision(): number {
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    const unlisten = onTimelineChanged(() => setRevision((r) => r + 1));
    return () => void unlisten.then((stop) => stop());
  }, []);
  return revision;
}
