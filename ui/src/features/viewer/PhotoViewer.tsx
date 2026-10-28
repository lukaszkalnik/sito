import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { PhotoFormat, Series, TimeSource, TimelineEntry } from '../timeline/api';
import { FORMAT_BADGES } from '../timeline/formatBadges';
import { thumbnailUrl } from '../timeline/thumbnails';
import { photoUrl, setWindowTitle, type ViewMode } from './api';
import { useShortcuts } from '../../shortcuts/useShortcuts';
import { formatDay, timeOf } from '../timeline/format';
import { dayOf } from '../timeline/layout';
import { navigationOrder, neighbours, seriesOf } from './navigation';
import './PhotoViewer.css';

interface PhotoViewerProps {
  entries: readonly TimelineEntry[];
  series: readonly Series[];
  mode: ViewMode;
  scanId: number;
  /** Timeline position of the photo shown. */
  position: number;
  onShow: (position: number) => void;
  onClose: () => void;
  /** Moves the shown photo into the "Odrzucone" folder. */
  onReject: () => void;
  /** Whether "Cofnij" has a rejection to undo. */
  canUndo: boolean;
  onUndo: () => void;
}

/**
 * The photo at full size. The previous photo stays until the next one has
 * loaded, so stepping through large photos never flashes an empty window;
 * if the next one cannot be loaded, only the error is shown.
 */
function FullPhoto({ src, alt }: { src: string; alt: string }) {
  const { t } = useTranslation();
  const [loaded, setLoaded] = useState<string | null>(null);
  const [failed, setFailed] = useState<string | null>(null);

  return (
    <div className="viewer-photo-area">
      {loaded !== null && failed !== src && <img className="viewer-photo" src={loaded} alt={alt} />}
      {loaded !== src && failed !== src && (
        <img
          className="viewer-photo viewer-photo--loading"
          src={src}
          alt=""
          data-testid="viewer-photo-loading"
          onLoad={() => setLoaded(src)}
          onError={() => setFailed(src)}
        />
      )}
      {failed === src && (
        <p role="alert" className="viewer-error">
          {t('viewer.loadError')}
        </p>
      )}
    </div>
  );
}

/** Arrow keys step, Escape closes, Delete rejects (keys in `shortcuts/keymap.ts`). */
function useViewerKeys(
  previous: number | null,
  next: number | null,
  { onShow, onClose, onReject }: Pick<PhotoViewerProps, 'onShow' | 'onClose' | 'onReject'>,
): void {
  useShortcuts({
    previous: previous === null ? null : () => onShow(previous),
    next: next === null ? null : () => onShow(next),
    close: onClose,
    reject: onReject,
  });
}

/** Fetches the photos `nextId` and `previousId` ahead of time, so stepping to them is instant. */
function usePreload(scanId: number, nextId: number | undefined, previousId: number | undefined) {
  useEffect(() => {
    for (const id of [nextId, previousId]) {
      if (id !== undefined) new Image().src = photoUrl(scanId, id);
    }
  }, [scanId, nextId, previousId]);
}

/** Name, date and folder of the photo, with the reject, undo and close buttons. */
/** Translation keys of the note on where a photo's date comes from; `null` for none. */
const DATE_SOURCE_NOTES: Readonly<Record<TimeSource, string | null>> = {
  exif: null,
  file: null,
  takeout: 'viewer.takeoutDate',
  takeoutCorrected: 'viewer.takeoutCorrected',
};

function DateSource({ source }: { source: TimeSource }) {
  const { t } = useTranslation();
  const note = DATE_SOURCE_NOTES[source];
  return note === null ? null : <span className="viewer-date-source">{t(note)}</span>;
}

function FormatNote({ format }: { format: PhotoFormat }) {
  const { t } = useTranslation();
  const badge = FORMAT_BADGES[format];
  return badge === null ? null : <span className="viewer-format">{t(badge)}</span>;
}

function ViewerBar({
  entry,
  onReject,
  canUndo,
  onUndo,
  onClose,
}: { entry: TimelineEntry } & Pick<
  PhotoViewerProps,
  'onReject' | 'canUndo' | 'onUndo' | 'onClose'
>) {
  const { t } = useTranslation();
  return (
    <header className="viewer-bar">
      <p className="viewer-info">
        <span className="viewer-name">{entry.name}</span>
        <span>
          {formatDay(dayOf(entry))}, {timeOf(entry.takenAt)}
        </span>
        <DateSource source={entry.timeSource} />
        <FormatNote format={entry.format} />
        <span className="viewer-folder">{entry.relativeDir || t('timeline.rootFolder')}</span>
      </p>
      <button type="button" className="viewer-reject" title={t('reject.hint')} onClick={onReject}>
        {t('reject.button')}
      </button>
      <button
        type="button"
        className="viewer-close"
        title={t('undo.hint')}
        disabled={!canUndo}
        onClick={onUndo}
      >
        {t('undo.button')}
      </button>
      <button type="button" className="viewer-close" onClick={onClose}>
        {t('viewer.close')}
      </button>
    </header>
  );
}

/** A previous/next button stepping to `target`, disabled when there is none. */
function StepButton({
  target,
  label,
  symbol,
  side,
  onShow,
}: {
  target: number | null;
  label: string;
  symbol: string;
  side: 'previous' | 'next';
  onShow: (position: number) => void;
}) {
  return (
    <button
      type="button"
      className={`viewer-step viewer-step--${side}`}
      aria-label={label}
      disabled={target === null}
      onClick={() => target !== null && onShow(target)}
    >
      {symbol}
    </button>
  );
}

/** Thumbnails of the series of the photo shown, the current one marked. */
function SeriesStrip({
  entries,
  strip,
  scanId,
  position,
  onShow,
}: {
  entries: readonly TimelineEntry[];
  strip: Series;
  scanId: number;
  position: number;
  onShow: (position: number) => void;
}) {
  const { t } = useTranslation();
  return (
    <nav className="viewer-strip" aria-label={t('viewer.series')}>
      {entries.slice(strip.start, strip.start + strip.len).map((member, i) => {
        const memberPosition = strip.start + i;
        const current = memberPosition === position;
        return (
          <button
            key={member.id}
            type="button"
            className={`viewer-strip-item${current ? ' viewer-strip-item--current' : ''}`}
            aria-label={member.name}
            aria-current={current ? 'true' : undefined}
            onClick={() => onShow(memberPosition)}
          >
            <img src={thumbnailUrl(scanId, member.id)} alt="" />
          </button>
        );
      })}
    </nav>
  );
}

/** The id of the photo at `position`, if there is one. */
function idAt(entries: readonly TimelineEntry[], position: number | null): number | undefined {
  return position === null ? undefined : entries[position]?.id;
}

/** Full-size photo with previous/next navigation and the thumbnails of its series. */
export function PhotoViewer(props: PhotoViewerProps) {
  const { entries, series, mode, scanId, position, onShow } = props;
  const { t } = useTranslation();
  const entry = entries[position];
  const order = useMemo(
    () => navigationOrder(entries.length, series, mode),
    [entries.length, series, mode],
  );
  const { previous, next } = neighbours(order, position);
  const strip = seriesOf(series, position);

  useEffect(() => {
    if (entry) void setWindowTitle(`${entry.name} – Sito`);
  }, [entry]);
  useViewerKeys(previous, next, props);
  usePreload(scanId, idAt(entries, next), idAt(entries, previous));

  if (!entry) return null;

  return (
    <main className="viewer">
      <ViewerBar entry={entry} {...props} />
      {/* The step buttons are centred on this whole area, strip included, so
          they stay in place whether or not the photo has a series strip. */}
      <div className="viewer-body">
        <StepButton
          target={previous}
          label={t('viewer.previous')}
          symbol="‹"
          side="previous"
          onShow={onShow}
        />
        <div className="viewer-stage">
          <FullPhoto src={photoUrl(scanId, entry.id)} alt={entry.name} />
        </div>
        <StepButton target={next} label={t('viewer.next')} symbol="›" side="next" onShow={onShow} />
        {strip && (
          <SeriesStrip
            entries={entries}
            strip={strip}
            scanId={scanId}
            position={position}
            onShow={onShow}
          />
        )}
      </div>
    </main>
  );
}
