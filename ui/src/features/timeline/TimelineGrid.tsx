import { useVirtualizer } from '@tanstack/react-virtual';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { TimelineEntry } from '../../api/scan';
import { formatDay, timeOf } from './format';
import { buildRows, columnsFor, rowHeight } from './layout';
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

/** Virtualized grid of photos grouped by day; renders only visible rows. */
export function TimelineGrid({ entries }: { entries: readonly TimelineEntry[] }) {
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);
  const width = useElementWidth(scrollRef);
  const rows = useMemo(() => buildRows(entries, columnsFor(width)), [entries, width]);

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
              className="timeline-row"
              style={{ height: item.size, transform: `translateY(${item.start}px)` }}
            >
              {row.kind === 'day' ? (
                <h2 className="timeline-day">{formatDay(row.date)}</h2>
              ) : (
                <ul className="timeline-tiles">
                  {row.entries.map((entry) => (
                    <li
                      key={entry.id}
                      className="timeline-tile"
                      title={entry.relativePath}
                      data-testid="timeline-tile"
                    >
                      <span className="timeline-tile-time">{timeOf(entry.takenAt)}</span>
                      <span className="timeline-tile-name">{entry.name}</span>
                      {entry.fromFileDate && (
                        <span className="timeline-tile-badge" title={t('timeline.fileDateHint')}>
                          {t('timeline.fileDate')}
                        </span>
                      )}
                    </li>
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
