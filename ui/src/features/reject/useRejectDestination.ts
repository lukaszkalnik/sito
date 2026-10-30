import { useCallback, useEffect, useState } from 'react';

import {
  loadDestination,
  onDestinationChanged,
  saveDestination,
  type RejectDestination,
} from './destination';

/**
 * The remembered reject destination and a function that changes it; kept in
 * sync with the other windows.
 */
export function useRejectDestination() {
  const [destination, setDestination] = useState<RejectDestination>(loadDestination);
  useEffect(() => onDestinationChanged(setDestination), []);
  const change = useCallback((next: RejectDestination) => {
    saveDestination(next);
    setDestination(next);
  }, []);
  return [destination, change] as const;
}
