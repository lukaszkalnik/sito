import { useVirtualizer } from '@tanstack/react-virtual';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { Series, TimelineEntry } from '../../api/scan';
import { formatDay, timeOf } from './format';
import { buildRows, columnsFor, rowHeight, type TimelineMode, type TimelineRow } from './layout';
import { TimelineTile } from './TimelineTile';
import './TimelineGrid.css';

/** Tracks an element's content width; 0 until it has been measured. */
function useElementWidth(ref: React.RefObject<HTMLElement | null>): number {
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const element = ref.current;
    if (!element || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(([first]) => {
      if (first) setWidth(first.contentRect.width);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref]);
  return width;
}

/** Header above the photos of one series: count and time range. */
function SeriesHeader({ entries }: { entries: readonly TimelineEntry[] }) {
  const { t } = useTranslation();
  const newest = entries[0];
  const oldest = entries[entries.length - 1];
  return (
    <h3 className="timeline-series">
      {t('series.header', {
        photos: t('scan.photoCount', { count: entries.length }),
        from: oldest ? timeOf(oldest.takenAt) : '',
        to: newest ? timeOf(newest.takenAt) : '',
      })}
    </h3>
  );
}

function rowClass(row: TimelineRow): string {
  if (row.kind === 'series') return 'timeline-row timeline-row--series timeline-row--series-start';
  if (row.kind === 'tiles' && row.inSeries) {
    return `timeline-row timeline-row--series${row.seriesEnd ? ' timeline-row--series-end' : ''}`;
  }
  return 'timeline-row';
}

interface TimelineGridProps {
  entries: readonly TimelineEntry[];
  series: readonly Series[];
  mode: TimelineMode;
  scanId: number;
}

/** Virtualized grid of photos grouped by day and series; renders only visible rows. */
export function TimelineGrid({ entries, series, mode, scanId }: TimelineGridProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const width = useElementWidth(scrollRef);
  const rows = useMemo(
    () => buildRows(entries, series, columnsFor(width), mode),
    [entries, series, width, mode],
  );

  // The app does not use the React Compiler, so its memoization caveat does not apply.
  // oxlint-disable-next-line react/incompatible-library
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (index) => {
      const row = rows[index];
      return row ? rowHeight(row) : 0;
    },
    getItemKey: (index) => rows[index]?.key ?? index,
    overscan: 4,
  });

  useEffect(() => {
    virtualizer.measure();
  }, [rows, virtualizer]);

  return (
    <div ref={scrollRef} className="timeline" data-testid="timeline">
      <div className="timeline-canvas" style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((item) => {
          const row = rows[item.index];
          if (!row) return null;
          return (
            <div
              key={item.key}
              className={rowClass(row)}
              style={{ height: item.size, transform: `translateY(${item.start}px)` }}
            >
              {row.kind === 'day' && <h2 className="timeline-day">{formatDay(row.date)}</h2>}
              {row.kind === 'series' && <SeriesHeader entries={row.entries} />}
              {row.kind === 'tiles' && (
                <ul className="timeline-tiles">
                  {row.entries.map((entry) => (
                    <TimelineTile key={entry.id} entry={entry} scanId={scanId} />
                  ))}
                </ul>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
