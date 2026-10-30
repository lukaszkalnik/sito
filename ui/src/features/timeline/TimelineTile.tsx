import { type SyntheticEvent, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { onFirstClick } from '../../clicks/firstClick';
import { AcceptedBadge } from '../accept/AcceptedBadge';
import type { TimelineEntry } from './api';
import { FORMAT_BADGES } from './formatBadges';
import type { RevealReason } from './reveal';
import { thumbnailUrl } from './thumbnails';
import { timeOf } from './format';
import { rememberLoaded, shapeFrom, shapeOf, type ThumbnailShape } from './loadedThumbnails';
import { useThumbnailTurn } from './useThumbnailTurn';

type ThumbnailStatus = 'loading' | 'loaded' | 'error';

/** How loading the thumbnail at `src` ended. */
type ThumbnailResult =
  { src: string; status: 'loaded'; shape: ThumbnailShape } | { src: string; status: 'error' };

interface TimelineTileProps {
  entry: TimelineEntry;
  scanId: number;
  /** Show the photo's folder, to tell same-named photos from different folders apart. */
  showFolder: boolean;
  /** Opens the photo full size. */
  onOpen: (entry: TimelineEntry) => void;
  /** Briefly marked for this reason, e.g. right after it was restored; `null` when not. */
  highlight?: RevealReason | null;
  /** Framed while the viewer shows it. */
  followed?: boolean;
  /** Marked with a check: the user accepted it. */
  accepted?: boolean;
}

/**
 * Load status of the thumbnail at `src`, and the handlers that update it. A
 * thumbnail seen before is cached by the webview, so it counts as loaded (and
 * `instant`) at once. `settled` is called once the thumbnail has loaded or failed.
 */
function useThumbnailStatus(src: string, settled: () => void) {
  // Remembers which URL the status belongs to, so a reused tile starts fresh.
  const [result, setResult] = useState<ThumbnailResult | null>(null);
  const remembered = shapeOf(src);
  const instant = remembered !== undefined;
  const current = result?.src === src ? result : null;
  const status: ThumbnailStatus = current ? current.status : instant ? 'loaded' : 'loading';
  const shape = (current?.status === 'loaded' ? current.shape : remembered) ?? 'landscape';
  const onLoad = (event: SyntheticEvent<HTMLImageElement>) => {
    const image = event.currentTarget;
    const loadedShape = shapeFrom(image.naturalWidth, image.naturalHeight);
    settled();
    rememberLoaded(src, loadedShape);
    setResult({ src, status: 'loaded', shape: loadedShape });
  };
  const onError = () => {
    settled();
    setResult({ src, status: 'error' });
  };
  return { status, shape, instant, onLoad, onError };
}

/** Icon shown in place of a thumbnail that could not be loaded. */
function BrokenThumbnail() {
  const { t } = useTranslation();
  return (
    <svg
      className="timeline-tile-broken"
      role="img"
      aria-label={t('timeline.thumbnailError')}
      viewBox="0 0 24 24"
      width="24"
      height="24"
    >
      <title>{t('timeline.thumbnailError')}</title>
      <path
        d="M4 5h16v14H4zM4 15l5-5 4 4 2-2 5 5M3 3l18 18"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

/** Capture time, file name and (optionally) folder below a thumbnail. */
function TileCaption({ entry, showFolder }: { entry: TimelineEntry; showFolder: boolean }) {
  const { t } = useTranslation();
  return (
    <span className="timeline-tile-caption">
      <span className="timeline-tile-time">{timeOf(entry.takenAt)}</span>
      <span className="timeline-tile-name">{entry.name}</span>
      {showFolder && (
        <span className="timeline-tile-folder" data-testid="timeline-tile-folder">
          <bdi>{entry.relativeDir || t('timeline.rootFolder')}</bdi>
        </span>
      )}
    </span>
  );
}

/** How a tile looks besides its thumbnail. */
interface TileLook {
  status: ThumbnailStatus;
  shape: ThumbnailShape;
  instant: boolean;
  highlight: RevealReason | null;
  followed: boolean;
  accepted: boolean;
}

/** The CSS classes of a tile. */
function tileClass({ status, shape, instant, highlight, followed, accepted }: TileLook): string {
  return [
    'timeline-tile',
    `timeline-tile--${status}`,
    shape === 'portrait' && 'timeline-tile--portrait',
    instant && 'timeline-tile--instant',
    highlight !== null && 'timeline-tile--highlighted',
    highlight === 'viewerClosed' && 'timeline-tile--highlighted-long',
    followed && 'timeline-tile--followed',
    accepted && 'timeline-tile--accepted',
  ]
    .filter(Boolean)
    .join(' ');
}

function TileBadges({ entry }: { entry: TimelineEntry }) {
  const { t } = useTranslation();
  const format = FORMAT_BADGES[entry.format];
  const fileDate = entry.timeSource === 'file';
  if (format === null && !fileDate) return null;
  return (
    <span className="timeline-tile-badges">
      {format !== null && <span className="timeline-tile-badge">{t(format)}</span>}
      {fileDate && (
        <span className="timeline-tile-badge" title={t('timeline.fileDateHint')}>
          {t('timeline.fileDate')}
        </span>
      )}
    </span>
  );
}

/** One photo on the timeline: its thumbnail with capture time, file name and folder. */
export function TimelineTile({
  entry,
  scanId,
  showFolder,
  onOpen,
  highlight = null,
  followed = false,
  accepted = false,
}: TimelineTileProps) {
  const { t } = useTranslation();
  const tileRef = useRef<HTMLLIElement>(null);
  const src = thumbnailUrl(scanId, entry.id);
  const { turn, finished } = useThumbnailTurn(tileRef, src);
  const { status, shape, instant, onLoad, onError } = useThumbnailStatus(src, finished);

  return (
    <li
      ref={tileRef}
      className={tileClass({ status, shape, instant, highlight, followed, accepted })}
      aria-current={followed ? 'true' : undefined}
      data-shape={shape}
      title={entry.relativePath}
      data-testid="timeline-tile"
    >
      {(turn || instant) && status !== 'error' && (
        <img
          className="timeline-tile-image"
          src={src}
          alt=""
          decoding={instant ? 'sync' : 'async'}
          onLoad={onLoad}
          onError={onError}
        />
      )}
      {status === 'error' && <BrokenThumbnail />}
      <TileCaption entry={entry} showFolder={showFolder} />
      <TileBadges entry={entry} />
      {accepted && <AcceptedBadge />}
      {/* Covers the whole tile, so a click or Enter anywhere opens the photo. */}
      <button
        type="button"
        className="timeline-tile-open"
        aria-label={t('timeline.open', { name: entry.name })}
        onClick={onFirstClick(() => onOpen(entry))}
      />
    </li>
  );
}
