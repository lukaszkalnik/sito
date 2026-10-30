import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

import type { SeriesLabel, TimelineEntry } from './api';
import { formatDay, timeOf } from './format';
import type { RowGroup, TimelineRow } from './layout';
import type { Reveal } from './reveal';
import { TimelineTile } from './TimelineTile';

/** What every tile of the grid needs besides its own photo. */
export interface TileContext {
  scanId: number;
  /** Whether tiles name their folder. */
  showFolder: boolean;
  /** Opens a photo full size. */
  onOpen: (entry: TimelineEntry) => void;
  /** The accepted photos, marked with a check. */
  acceptedIds: ReadonlySet<number>;
  /** The photo to highlight, if any. */
  highlighted: Reveal | null;
  /** The photo the viewer shows, if any. */
  followed: number | null;
}

/** The camera a series header names, if any. */
function cameraOf(label: SeriesLabel, t: TFunction): string | null {
  switch (label.kind) {
    case 'plain':
      return null;
    case 'camera':
      return label.name;
    case 'unknownCamera':
      return t('series.unknownCamera');
  }
}

/** Header above the photos of one series: count, time range and, when it overlaps another camera's series, its camera. */
function SeriesHeader({
  entries,
  label,
}: {
  entries: readonly TimelineEntry[];
  label: SeriesLabel;
}) {
  const { t } = useTranslation();
  const newest = entries[0];
  const oldest = entries[entries.length - 1];
  const from = oldest ? timeOf(oldest.takenAt) : '';
  const to = newest ? timeOf(newest.takenAt) : '';
  const photos = t('scan.photoCount', { count: entries.length });
  const header =
    from === to
      ? t('series.headerAt', { photos, time: to })
      : t('series.header', { photos, from, to });
  const camera = cameraOf(label, t);
  const text = camera === null ? header : t('series.withCamera', { header, camera });
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
          highlight={entry.id === context.highlighted?.id ? context.highlighted.reason : null}
          followed={entry.id === context.followed}
          accepted={context.acceptedIds.has(entry.id)}
        />
      ))}
    </ul>
  );
}

/** One group of a row of blocks: a series block with a header, or single photos. */
function BlockGroup({ group, context }: { group: RowGroup; context: TileContext }) {
  const tiles = <TileList entries={group.entries} context={context} />;
  if (group.kind === 'singles') return <div className="timeline-singles">{tiles}</div>;
  return (
    <section className="timeline-block">
      <SeriesHeader entries={group.entries} label={group.label} />
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
      return <SeriesHeader entries={row.entries} label={row.label} />;
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
