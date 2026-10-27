import { useEffect, useState } from 'react';

/**
 * Becomes `true` once the component has stayed mounted for `delayMs`.
 * Virtualized tiles unmount as soon as they scroll away, so tiles that only
 * fly past during fast scrolling never start loading their thumbnail.
 */
export function useMountedFor(delayMs: number): boolean {
  const [elapsed, setElapsed] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => setElapsed(true), delayMs);
    return () => clearTimeout(timer);
  }, [delayMs]);

  return elapsed;
}
