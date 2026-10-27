import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { TimelineEntry } from '../../api/scan';
import { thumbnailUrl } from '../../api/thumbnails';
import { timeOf } from './format';
import { rememberLoaded, wasLoaded } from './loadedThumbnails';
import { useMountedFor } from './useMountedFor';

/** How long a tile must stay on screen before its thumbnail is requested. */
export const THUMBNAIL_DELAY_MS = 150;

type ThumbnailStatus = 'loading' | 'loaded' | 'error';

interface TimelineTileProps {
  entry: TimelineEntry;
  scanId: number;
  /** Show the photo's folder, to tell same-named photos from different folders apart. */
  showFolder: boolean;
  /** Opens the photo full size. */
  onOpen: (entry: TimelineEntry) => void;
}

/** One photo on the timeline: its thumbnail with capture time, file name and folder. */
export function TimelineTile({ entry, scanId, showFolder, onOpen }: TimelineTileProps) {
  const { t } = useTranslation();
  const delayPassed = useMountedFor(THUMBNAIL_DELAY_MS);
  const src = thumbnailUrl(scanId, entry.id);
  // Remembers which URL the status belongs to, so a reused tile starts fresh.
  const [result, setResult] = useState<{ src: string; status: ThumbnailStatus } | null>(null);
  // A thumbnail seen before is cached by the webview: show it at once, unfaded.
  const seenBefore = wasLoaded(src);
  const status = result?.src === src ? result.status : seenBefore ? 'loaded' : 'loading';
  const showThumbnail = delayPassed || seenBefore;

  return (
    <li
      className={`timeline-tile timeline-tile--${status}${seenBefore ? ' timeline-tile--instant' : ''}`}
      title={entry.relativePath}
      data-testid="timeline-tile"
    >
      {showThumbnail && status !== 'error' && (
        <img
          className="timeline-tile-image"
          src={src}
          alt=""
          decoding={seenBefore ? 'sync' : 'async'}
          onLoad={() => {
            rememberLoaded(src);
            setResult({ src, status: 'loaded' });
          }}
          onError={() => setResult({ src, status: 'error' })}
        />
      )}
      {status === 'error' && (
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
      )}
      <span className="timeline-tile-caption">
        <span className="timeline-tile-time">{timeOf(entry.takenAt)}</span>
        <span className="timeline-tile-name">{entry.name}</span>
        {showFolder && (
          <span className="timeline-tile-folder" data-testid="timeline-tile-folder">
            <bdi>{entry.relativeDir || t('timeline.rootFolder')}</bdi>
          </span>
        )}
      </span>
      {entry.fromFileDate && (
        <span className="timeline-tile-badge" title={t('timeline.fileDateHint')}>
          {t('timeline.fileDate')}
        </span>
      )}
      {/* Covers the whole tile, so a click or Enter anywhere opens the photo. */}
      <button
        type="button"
        className="timeline-tile-open"
        aria-label={t('timeline.open', { name: entry.name })}
        onClick={() => onOpen(entry)}
      />
    </li>
  );
}
