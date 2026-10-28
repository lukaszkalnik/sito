import { useTranslation } from 'react-i18next';

import type { TimelineEntry } from '../../api/scan';
import { formatDay, timeOf } from './format';
import type { RowGroup, TimelineRow } from './layout';
import { TimelineTile } from './TimelineTile';

/** What every tile of the grid needs besides its own photo. */
export interface TileContext {
  scanId: number;
  /** Whether tiles name their folder. */
  showFolder: boolean;
  /** Opens a photo full size. */
  onOpen: (entry: TimelineEntry) => void;
  /** The photo to highlight, if any. */
  highlighted: number | null;
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

/** A list of photo tiles. */
function TileList({
  entries,
  context,
}: {
  entries: readonly TimelineEntry[];
  context: TileContext;
}) {
  return (
    <ul className="timeline-tiles">
      {entries.map((entry) => (
        <TimelineTile
          key={entry.id}
          entry={entry}
          scanId={context.scanId}
          showFolder={context.showFolder}
          onOpen={context.onOpen}
          highlighted={entry.id === context.highlighted}
        />
      ))}
    </ul>
  );
}

/** One group of a row of blocks: a series block with a header, or single photos. */
function BlockGroup({ group, context }: { group: RowGroup; context: TileContext }) {
  const tiles = <TileList entries={group.entries} context={context} />;
  if (!group.series) return <div className="timeline-singles">{tiles}</div>;
  return (
    <section className="timeline-block">
      <SeriesHeader entries={group.entries} />
      {tiles}
    </section>
  );
}

/** The content of one timeline row. */
export function TimelineRowView({ row, context }: { row: TimelineRow; context: TileContext }) {
  switch (row.kind) {
    case 'day':
      return <h2 className="timeline-day">{formatDay(row.date)}</h2>;
    case 'series':
      return <SeriesHeader entries={row.entries} />;
    case 'tiles':
      return <TileList entries={row.entries} context={context} />;
    case 'blocks':
      return (
        <div className="timeline-blocks">
          {row.groups.map((group) => (
            <BlockGroup key={group.entries[0]?.id} group={group} context={context} />
          ))}
        </div>
      );
  }
}
