import { useVirtualizer } from '@tanstack/react-virtual';
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { Series, TimelineEntry } from '../../api/scan';
import { formatDay, timeOf } from './format';
import {
  anchoredScrollTop,
  blockWidth,
  buildRows,
  columnsFor,
  revealScrollTop,
  rowHeight,
  spansSeveralFolders,
  totalHeight,
  type TimelineMode,
  type TimelineRow,
} from './layout';
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
  const from = oldest ? timeOf(oldest.takenAt) : '';
  const to = newest ? timeOf(newest.takenAt) : '';
  const photos = t('scan.photoCount', { count: entries.length });
  const text =
    from === to
      ? t('series.headerAt', { photos, time: to })
      : t('series.header', { photos, from, to });
  return (
    <h3 className="timeline-series" title={text}>
      {text}
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
  /** Opens a photo full size. */
  onOpen: (entry: TimelineEntry) => void;
  /** A photo to scroll into view and highlight once the rows show it, e.g. one just restored. */
  reveal?: number | null;
  /** Called when `reveal` was shown (or could not be). */
  onRevealed?: () => void;
}

/** How long a revealed photo stays highlighted. */
export const HIGHLIGHT_MS = 2600;
/** How long to wait for a revealed photo to appear in the rows. */
const REVEAL_WAIT_MS = 2000;

/** Virtualized grid of photos grouped by day and series; renders only visible rows. */
export function TimelineGrid({
  entries,
  series,
  mode,
  scanId,
  onOpen,
  reveal = null,
  onRevealed,
}: TimelineGridProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const width = useElementWidth(scrollRef);
  const rows = useMemo(() => {
    const columns = columnsFor(width);
    // Series blocks may use all of the measured width, not only whole columns.
    return buildRows(entries, series, columns, mode, Math.max(width, blockWidth(columns)));
  }, [entries, series, width, mode]);
  // Folders only tell photos apart when the scan spans more than one.
  const showFolder = useMemo(() => spansSeveralFolders(entries), [entries]);

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

  // When the rows change (new gap, mode or width), keep the photos at the top
  // of the view in place. Runs before paint, so the grid never visibly jumps.
  const previousRows = useRef(rows);
  // The scroll position in the old rows. Read from scroll events, because by
  // the time the effect runs, a shorter new canvas may already have made the
  // browser clamp `scrollTop`.
  const lastScrollTop = useRef(0);
  useLayoutEffect(() => {
    const oldRows = previousRows.current;
    previousRows.current = rows;
    virtualizer.measure();
    const element = scrollRef.current;
    if (element && oldRows !== rows) {
      element.scrollTop = anchoredScrollTop(oldRows, rows, lastScrollTop.current);
      lastScrollTop.current = element.scrollTop;
    }
  }, [rows, virtualizer]);

  // Scrolls to a photo to reveal as soon as the rows show it. The rows may
  // still be the old ones (a restored photo arrives with the next reload), so
  // it waits a moment before giving up.
  const [highlighted, setHighlighted] = useState<number | null>(null);
  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (reveal === null || !element) return;
    const target = revealScrollTop(rows, reveal, element.scrollTop, element.clientHeight);
    if (target === null) {
      const timer = setTimeout(() => onRevealed?.(), REVEAL_WAIT_MS);
      return () => clearTimeout(timer);
    }
    element.scrollTop = target;
    lastScrollTop.current = element.scrollTop;
    setHighlighted(reveal);
    onRevealed?.();
  }, [rows, reveal, onRevealed]);
  useEffect(() => {
    if (highlighted === null) return;
    const timer = setTimeout(() => setHighlighted(null), HIGHLIGHT_MS);
    return () => clearTimeout(timer);
  }, [highlighted]);

  return (
    <div
      ref={scrollRef}
      className="timeline"
      data-testid="timeline"
      onScroll={(event) => {
        lastScrollTop.current = event.currentTarget.scrollTop;
      }}
    >
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
              {row.kind === 'day' && <h2 className="timeline-day">{formatDay(row.date)}</h2>}
              {row.kind === 'series' && <SeriesHeader entries={row.entries} />}
              {row.kind === 'tiles' && (
                <ul className="timeline-tiles">
                  {row.entries.map((entry) => (
                    <TimelineTile
                      key={entry.id}
                      entry={entry}
                      scanId={scanId}
                      showFolder={showFolder}
                      onOpen={onOpen}
                      highlighted={entry.id === highlighted}
                    />
                  ))}
                </ul>
              )}
              {row.kind === 'blocks' && (
                <div className="timeline-blocks">
                  {row.groups.map((group) => {
                    const tiles = (
                      <ul className="timeline-tiles">
                        {group.entries.map((entry) => (
                          <TimelineTile
                            key={entry.id}
                            entry={entry}
                            scanId={scanId}
                            showFolder={showFolder}
                            onOpen={onOpen}
                            highlighted={entry.id === highlighted}
                          />
                        ))}
                      </ul>
                    );
                    const key = group.entries[0]?.id;
                    return group.series ? (
                      <section key={key} className="timeline-block">
                        <SeriesHeader entries={group.entries} />
                        {tiles}
                      </section>
                    ) : (
                      <div key={key} className="timeline-singles">
                        {tiles}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
