import type { TimelineEntry } from './api';

/**
 * The entries in display order: `order` lists their indices in the order they
 * are shown. `null` when `order` does not name every entry exactly once (e.g.
 * series found on a newer timeline than the entries).
 */
export function arrangeEntries(
  entries: readonly TimelineEntry[],
  order: readonly number[],
): TimelineEntry[] | null {
  if (order.length !== entries.length) return null;
  const seen = new Uint8Array(entries.length);
  const arranged: TimelineEntry[] = [];
  for (const index of order) {
    const entry = entries[index];
    if (entry === undefined || seen[index] === 1) return null;
    seen[index] = 1;
    arranged.push(entry);
  }
  return arranged;
}
