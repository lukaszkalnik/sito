import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { Series, TimelineEntry } from '../../api/scan';
import { thumbnailUrl } from '../../api/thumbnails';
import { photoUrl, setWindowTitle, type ViewMode } from '../../api/viewer';
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
 * loaded, so stepping through large photos never flashes an empty window.
 */
function FullPhoto({ src, alt }: { src: string; alt: string }) {
  const { t } = useTranslation();
  const [loaded, setLoaded] = useState<string | null>(null);
  const [failed, setFailed] = useState<string | null>(null);

  return (
    <div className="viewer-photo-area">
      {loaded !== null && <img className="viewer-photo" src={loaded} alt={alt} />}
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

/** Full-size photo with previous/next navigation and the thumbnails of its series. */
export function PhotoViewer({
  entries,
  series,
  mode,
  scanId,
  position,
  onShow,
  onClose,
  onReject,
  canUndo,
  onUndo,
}: PhotoViewerProps) {
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

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'ArrowLeft' && previous !== null) onShow(previous);
      else if (event.key === 'ArrowRight' && next !== null) onShow(next);
      else if (event.key === 'Escape') onClose();
      else if (event.key === 'Delete') onReject();
      else return;
      event.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [previous, next, onShow, onClose, onReject]);

  // Fetch the neighbours ahead of time, so stepping to them is instant.
  useEffect(() => {
    for (const neighbour of [next, previous]) {
      const id = neighbour === null ? undefined : entries[neighbour]?.id;
      if (id !== undefined) new Image().src = photoUrl(scanId, id);
    }
  }, [next, previous, entries, scanId]);

  if (!entry) return null;

  return (
    <main className="viewer">
      <header className="viewer-bar">
        <p className="viewer-info">
          <span className="viewer-name">{entry.name}</span>
          <span>
            {formatDay(dayOf(entry))}, {timeOf(entry.takenAt)}
          </span>
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

      <div className="viewer-stage">
        <button
          type="button"
          className="viewer-step"
          aria-label={t('viewer.previous')}
          disabled={previous === null}
          onClick={() => previous !== null && onShow(previous)}
        >
          ‹
        </button>
        <FullPhoto src={photoUrl(scanId, entry.id)} alt={entry.name} />
        <button
          type="button"
          className="viewer-step"
          aria-label={t('viewer.next')}
          disabled={next === null}
          onClick={() => next !== null && onShow(next)}
        >
          ›
        </button>
      </div>

      {strip && (
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
      )}
    </main>
  );
}
