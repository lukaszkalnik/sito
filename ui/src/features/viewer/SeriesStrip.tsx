import { type Ref, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import { EditedIcon } from '../series-edit/EditedIcon';
import type { TimelineEntry } from '../timeline/api';
import { menuRequest, type MenuRequest } from '../timeline/menuRequest';
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

interface StripItemProps {
  member: TimelineEntry;
  current: boolean;
  scanId: number;
  accepted: boolean;
  pulsed: boolean;
  onShow: (id: number) => void;
  onMenu: (request: MenuRequest) => void;
}

/** One thumbnail of the strip. */
function StripItem({
  member,
  current,
  scanId,
  accepted,
  pulsed,
  onShow,
  onMenu,
  ref,
}: StripItemProps & { ref?: Ref<HTMLButtonElement> }) {
  const { t } = useTranslation();
  return (
    <button
      ref={ref}
      type="button"
      className={itemClass(current, pulsed)}
      aria-label={accepted ? t('accept.stripItem', { name: member.name }) : member.name}
      aria-current={current ? 'true' : undefined}
      onClick={() => onShow(member.id)}
      onContextMenu={(event) => {
        event.preventDefault();
        onMenu(menuRequest({ kind: 'photo', id: member.id }, event));
      }}
    >
      <img src={thumbnailUrl(scanId, member.id)} alt="" />
      {accepted && <AcceptedBadge />}
    </button>
  );
}

/**
 * Thumbnails of the series of the photo shown, the current one and the
 * accepted ones marked, and the `pulsedIds` pointed out with a few pulses.
 * A series edited by hand starts with the pencil; a right-click (or the Menu
 * key) on a thumbnail asks for the edit menu of that photo.
 */
export function SeriesStrip({
  members,
  currentId,
  scanId,
  acceptedIds,
  pulsedIds = NONE,
  edited,
  onShow,
  onMenu,
}: {
  members: readonly TimelineEntry[];
  currentId: number;
  scanId: number;
  acceptedIds: ReadonlySet<number>;
  pulsedIds?: ReadonlySet<number>;
  /** Whether the series was edited by hand. */
  edited: boolean;
  onShow: (id: number) => void;
  onMenu: (request: MenuRequest) => void;
}) {
  const { t } = useTranslation();
  const currentRef = useRef<HTMLButtonElement>(null);
  // Stepping through a long series would otherwise leave the strip behind.
  useEffect(() => {
    currentRef.current?.scrollIntoView({ inline: 'nearest', block: 'nearest' });
  }, [currentId]);
  return (
    <nav className="viewer-strip" aria-label={t('viewer.series')}>
      {edited && (
        <span className="viewer-strip-edited">
          <EditedIcon />
        </span>
      )}
      {members.map((member) => {
        const current = member.id === currentId;
        return (
          <StripItem
            key={member.id}
            ref={current ? currentRef : undefined}
            member={member}
            current={current}
            scanId={scanId}
            accepted={acceptedIds.has(member.id)}
            pulsed={pulsedIds.has(member.id)}
            onShow={onShow}
            onMenu={onMenu}
          />
        );
      })}
    </nav>
  );
}
