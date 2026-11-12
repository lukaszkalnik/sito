import { useTranslation } from 'react-i18next';

import { onFirstClick } from '../../clicks/firstClick';
import { AcceptButton } from '../accept/AcceptButton';
import type { UndoState } from '../reject/api';
import type { RejectDestination } from '../reject/destination';
import { DestinationIcon } from '../reject/DestinationIcon';
import { UndoButton } from '../reject/UndoButton';
import type { TimelineEntry } from '../timeline/api';
import type { PhotoDetails } from './api';
import { PhotoInfo } from './PhotoInfo';
import './ViewerBar.css';

/** What the viewer bar shows and does. */
export interface ViewerBarProps {
  onClose: () => void;
  /** Rejects the shown photo. */
  onReject: () => void;
  /** Where „Odrzuć” moves the photo, shown by its icon and tooltip. */
  rejectTo: RejectDestination;
  /** The accepted photos of the scan, marked in the bar and the series strip. */
  acceptedIds: ReadonlySet<number>;
  /** Accepts the shown photo, or takes its mark off. */
  onToggleAccept: () => void;
  /** What "Cofnij" can undo. */
  undoState: UndoState;
  onUndo: () => void;
}

/** Translation keys of the „Odrzuć” tooltip, per destination. */
const REJECT_HINT_KEYS: Readonly<Record<RejectDestination, string>> = {
  folder: 'reject.hint',
  trash: 'reject.hintTrash',
};

/** „Odrzuć”, with the icon of where it moves the photo: the folder or the trash. */
function RejectButton({ rejectTo, onReject }: Pick<ViewerBarProps, 'rejectTo' | 'onReject'>) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      className="viewer-reject"
      title={t(REJECT_HINT_KEYS[rejectTo])}
      onClick={onFirstClick(onReject)}
    >
      <DestinationIcon destination={rejectTo} />
      {t('reject.button')}
    </button>
  );
}

/** The photo's info, with the accept, reject, undo and close buttons. */
export function ViewerBar({
  entry,
  details,
  acceptedIds,
  onToggleAccept,
  onReject,
  rejectTo,
  undoState,
  onUndo,
  onClose,
}: { entry: TimelineEntry; details: PhotoDetails } & ViewerBarProps) {
  const { t } = useTranslation();
  return (
    <header className="viewer-bar">
      <PhotoInfo entry={entry} details={details} />
      <AcceptButton accepted={acceptedIds.has(entry.id)} onToggle={onToggleAccept} />
      <RejectButton rejectTo={rejectTo} onReject={onReject} />
      <UndoButton className="viewer-close viewer-close--icon" state={undoState} onUndo={onUndo} />
      <button type="button" className="viewer-close" onClick={onFirstClick(onClose)}>
        {t('viewer.close')}
      </button>
    </header>
  );
}
