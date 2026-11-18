import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

import { EditedIcon } from '../series-edit/EditedIcon';
import type { SeriesLabel, TimelineEntry } from './api';
import { formatDay, timeOf } from './format';
import type { RowGroup, TimelineRow } from './layout';
import { menuRequest, type MenuRequest } from './menuRequest';
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
  /** The photos taken out of their series by hand, marked with a pencil. */
  pinnedIds?: ReadonlySet<number>;
  /** The photo to highlight, if any. */
  highlighted: Reveal | null;
  /** The photo the viewer shows, if any. */
  followed: number | null;
  /** Counts the timeline's changes; failed thumbnails load again with each. */
  revision: number;
  /** Opens the context menu of a photo or series; without it there is none. */
  onMenu?: (request: MenuRequest) => void;
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

/** What a series header shows. */
interface SeriesHeaderProps {
  entries: readonly TimelineEntry[];
  label: SeriesLabel;
  /** Edited by hand: marked with a pencil. */
  edited: boolean;
  onMenu?: (request: MenuRequest) => void;
}

/** The text of a series header: count, time range and, when it overlaps another camera's series, its camera. */
function headerText(entries: readonly TimelineEntry[], label: SeriesLabel, t: TFunction): string {
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
  return camera === null ? header : t('series.withCamera', { header, camera });
}

/** Header above the photos of one series, with its context menu. */
function SeriesHeader({ entries, label, edited, onMenu }: SeriesHeaderProps) {
  const { t } = useTranslation();
  const text = headerText(entries, label, t);
  const first = entries[0];
  return (
    <h3
      className="timeline-series"
      title={text}
      onContextMenu={(event) => {
        if (!onMenu || !first) return;
        event.preventDefault();
        onMenu(menuRequest({ kind: 'series', firstId: first.id }, event));
      }}
    >
      <span className="timeline-series-text">{text}</span>
      {edited && <EditedIcon />}
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
          pinned={context.pinnedIds?.has(entry.id) ?? false}
          revision={context.revision}
          onMenu={context.onMenu}
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
      <SeriesHeader
        entries={group.entries}
        label={group.label}
        edited={group.edited}
        onMenu={context.onMenu}
      />
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
      return (
        <SeriesHeader
          entries={row.entries}
          label={row.label}
          edited={row.edited}
          onMenu={context.onMenu}
        />
      );
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
