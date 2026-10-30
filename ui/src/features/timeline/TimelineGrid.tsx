import { useVirtualizer } from '@tanstack/react-virtual';
import { useMemo, useRef, type RefObject } from 'react';

import type { Series, TimelineEntry } from './api';
import {
  blockWidth,
  buildRows,
  columnsFor,
  rowHeight,
  spansSeveralFolders,
  totalHeight,
  type TimelineMode,
  type TimelineRow,
} from './layout';
import { TimelineRowView } from './TimelineRowView';
import { useElementWidth, useGridScroll } from './useGridScroll';
import './TimelineGrid.css';

interface TimelineGridProps {
  entries: readonly TimelineEntry[];
  series: readonly Series[];
  mode: TimelineMode;
  scanId: number;
  /** The accepted photos, marked with a check. */
  acceptedIds: ReadonlySet<number>;
  /** Opens a photo full size. */
  onOpen: (entry: TimelineEntry) => void;
  /** A photo to scroll into view and highlight once the rows show it, e.g. one just restored. */
  reveal?: number | null;
  /** Called when `reveal` was shown (or could not be). */
  onRevealed?: () => void;
}

/** The rows of the grid for the measured `width`. */
function useRows(
  entries: readonly TimelineEntry[],
  series: readonly Series[],
  mode: TimelineMode,
  width: number,
): TimelineRow[] {
  return useMemo(() => {
    const columns = columnsFor(width);
    // Series blocks may use all of the measured width, not only whole columns.
    return buildRows(entries, series, columns, mode, Math.max(width, blockWidth(columns)));
  }, [entries, series, width, mode]);
}

/** Virtualizes `rows`, whose heights are known exactly. */
function useRowVirtualizer(
  rows: readonly TimelineRow[],
  scrollRef: RefObject<HTMLDivElement | null>,
) {
  // The app does not use the React Compiler, so its memoization caveat does not apply.
  // oxlint-disable-next-line react/incompatible-library
  return useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (index) => {
      const row = rows[index];
      return row ? rowHeight(row) : 0;
    },
    getItemKey: (index) => rows[index]?.key ?? index,
    overscan: 4,
  });
}

/** The CSS classes of a row's container. */
function rowClass(row: TimelineRow): string {
  if (row.kind === 'series') return 'timeline-row timeline-row--series timeline-row--series-start';
  if (row.kind === 'tiles' && row.inSeries) {
    return `timeline-row timeline-row--series${row.seriesEnd ? ' timeline-row--series-end' : ''}`;
  }
  return 'timeline-row';
}

/** Virtualized grid of photos grouped by day and series; renders only visible rows. */
export function TimelineGrid({
  entries,
  series,
  mode,
  scanId,
  acceptedIds,
  onOpen,
  reveal = null,
  onRevealed,
}: TimelineGridProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const rows = useRows(entries, series, mode, useElementWidth(scrollRef));
  // Folders only tell photos apart when the scan spans more than one.
  const showFolder = useMemo(() => spansSeveralFolders(entries), [entries]);
  const virtualizer = useRowVirtualizer(rows, scrollRef);
  const { highlighted, onScroll } = useGridScroll(scrollRef, rows, virtualizer, reveal, onRevealed);
  const context = { scanId, showFolder, onOpen, acceptedIds, highlighted };

  return (
    <div ref={scrollRef} className="timeline" data-testid="timeline" onScroll={onScroll}>
      {/* Row heights are exact, so the canvas is sized from the rows directly:
          the scroll position above can then be set before the virtualizer re-measures. */}
      <div className="timeline-canvas" style={{ height: totalHeight(rows) }}>
        {virtualizer.getVirtualItems().map((item) => {
          const row = rows[item.index];
          if (!row) return null;
          return (
            <div
              key={item.key}
              className={rowClass(row)}
              style={{ height: item.size, transform: `translateY(${item.start}px)` }}
            >
              <TimelineRowView row={row} context={context} />
            </div>
          );
        })}
      </div>
    </div>
  );
}
