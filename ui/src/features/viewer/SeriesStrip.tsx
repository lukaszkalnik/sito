import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import type { TimelineEntry } from '../timeline/api';
import { thumbnailUrl } from '../timeline/thumbnails';
import { AcceptedBadge } from '../accept/AcceptedBadge';

const NONE: ReadonlySet<number> = new Set();

/** The class of a strip item. */
function itemClass(current: boolean, pulsed: boolean): string {
  return [
    'viewer-strip-item',
    current && 'viewer-strip-item--current',
    pulsed && 'viewer-strip-item--pulsed',
  ]
    .filter(Boolean)
    .join(' ');
}

/**
 * Thumbnails of the series of the photo shown, the current one and the
 * accepted ones marked, and the `pulsedIds` pointed out with a few pulses.
 */
export function SeriesStrip({
  members,
  currentId,
  scanId,
  acceptedIds,
  pulsedIds = NONE,
  onShow,
}: {
  members: readonly TimelineEntry[];
  currentId: number;
  scanId: number;
  acceptedIds: ReadonlySet<number>;
  pulsedIds?: ReadonlySet<number>;
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
        const accepted = acceptedIds.has(member.id);
        return (
          <button
            key={member.id}
            ref={current ? currentRef : undefined}
            type="button"
            className={itemClass(current, pulsedIds.has(member.id))}
            aria-label={accepted ? t('accept.stripItem', { name: member.name }) : member.name}
            aria-current={current ? 'true' : undefined}
            onClick={() => onShow(member.id)}
          >
            <img src={thumbnailUrl(scanId, member.id)} alt="" />
            {accepted && <AcceptedBadge />}
          </button>
        );
      })}
    </nav>
  );
}
