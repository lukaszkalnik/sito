import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import type { TimelineEntry } from '../timeline/api';
import { thumbnailUrl } from '../timeline/thumbnails';

/** Thumbnails of the series of the photo shown, the current one marked. */
export function SeriesStrip({
  members,
  currentId,
  scanId,
  onShow,
}: {
  members: readonly TimelineEntry[];
  currentId: number;
  scanId: number;
  onShow: (id: number) => void;
}) {
  const { t } = useTranslation();
  const currentRef = useRef<HTMLButtonElement>(null);
  // Stepping through a long series would otherwise leave the strip behind.
  useEffect(() => {
    currentRef.current?.scrollIntoView({ inline: 'nearest', block: 'nearest' });
  }, [currentId]);
  return (
    <nav className="viewer-strip" aria-label={t('viewer.series')}>
      {members.map((member) => {
        const current = member.id === currentId;
        return (
          <button
            key={member.id}
            ref={current ? currentRef : undefined}
            type="button"
            className={`viewer-strip-item${current ? ' viewer-strip-item--current' : ''}`}
            aria-label={member.name}
            aria-current={current ? 'true' : undefined}
            onClick={() => onShow(member.id)}
          >
            <img src={thumbnailUrl(scanId, member.id)} alt="" />
          </button>
        );
      })}
    </nav>
  );
}
