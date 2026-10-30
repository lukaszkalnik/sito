import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';

import { onFirstClick } from '../../clicks/firstClick';
import type { PhotoFormat, TimeSource, TimelineEntry } from '../timeline/api';
import type { RejectDestination } from '../reject/destination';
import { DestinationIcon } from '../reject/DestinationIcon';
import { AcceptButton } from '../accept/AcceptButton';
import { FORMAT_BADGES } from '../timeline/formatBadges';
import { photoUrl, setWindowTitle, type PhotoDetails, type ViewerPhoto } from './api';
import { fileSize, formatPixels, type FileSize } from './photoDetails';
import { useShortcuts } from '../../shortcuts/useShortcuts';
import { formatDay, timeOf } from '../timeline/format';
import { dayOf } from '../timeline/layout';
import { FullPhoto } from './FullPhoto';
import { SeriesStrip } from './SeriesStrip';
import { usePhotoZoom } from './usePhotoZoom';
import './PhotoViewer.css';

interface PhotoViewerProps {
  /** The photo shown, with its series and neighbours. */
  photo: ViewerPhoto;
  scanId: number;
  /** Shows the photo with the given id. */
  onShow: (id: number) => void;
  onClose: () => void;
  /** Rejects the shown photo. */
  onReject: () => void;
  /** Where „Odrzuć” moves the photo, shown by its icon and tooltip. */
  rejectTo: RejectDestination;
  /** The accepted photos of the scan, marked in the bar and the series strip. */
  acceptedIds: ReadonlySet<number>;
  /** Accepts the shown photo, or takes its mark off. */
  onToggleAccept: () => void;
  /** Whether "Cofnij" has an action to undo. */
  canUndo: boolean;
  onUndo: () => void;
}

/** Arrow keys step, Escape closes, Delete rejects, A accepts (keys in `shortcuts/keymap.ts`). */
function useViewerKeys(
  previous: number | null,
  next: number | null,
  {
    onShow,
    onClose,
    onReject,
    onToggleAccept,
  }: Pick<PhotoViewerProps, 'onShow' | 'onClose' | 'onReject' | 'onToggleAccept'>,
): void {
  useShortcuts({
    previous: previous === null ? null : () => onShow(previous),
    next: next === null ? null : () => onShow(next),
    close: onClose,
    reject: onReject,
    accept: onToggleAccept,
  });
}

/** Fetches the photos `nextId` and `previousId` ahead of time, so stepping to them is instant. */
function usePreload(scanId: number, nextId: number | null, previousId: number | null) {
  useEffect(() => {
    for (const id of [nextId, previousId]) {
      if (id !== null) new Image().src = photoUrl(scanId, id);
    }
  }, [scanId, nextId, previousId]);
}

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

/** Translation keys of a file size, per unit. */
const SIZE_KEYS: Readonly<Record<FileSize['unit'], string>> = {
  mb: 'viewer.sizeMb',
  kb: 'viewer.sizeKb',
};

/** The photo's resolution (when known; not for RAW) and file size. */
function PhotoFacts({ details }: { details: PhotoDetails }) {
  const { t } = useTranslation();
  const size = fileSize(details.sizeBytes);
  const { resolution } = details;
  return (
    <>
      {resolution && (
        <span>
          {t('viewer.resolution', {
            width: formatPixels(resolution.width),
            height: formatPixels(resolution.height),
          })}
        </span>
      )}
      <span>{t(SIZE_KEYS[size.unit], { size: size.value })}</span>
    </>
  );
}

/** Translation keys of the „Odrzuć” tooltip, per destination. */
const REJECT_HINT_KEYS: Readonly<Record<RejectDestination, string>> = {
  folder: 'reject.hint',
  trash: 'reject.hintTrash',
};

/** „Odrzuć”, with the icon of where it moves the photo: the folder or the trash. */
function RejectButton({ rejectTo, onReject }: Pick<PhotoViewerProps, 'rejectTo' | 'onReject'>) {
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

/** Name, date, details and folder of the photo. */
function PhotoInfo({ entry, details }: { entry: TimelineEntry; details: PhotoDetails }) {
  const { t } = useTranslation();
  return (
    <p className="viewer-info">
      <span className="viewer-name">{entry.name}</span>
      <span>
        {formatDay(dayOf(entry))}, {timeOf(entry.takenAt)}
      </span>
      <PhotoFacts details={details} />
      <DateSource source={entry.timeSource} />
      <FormatNote format={entry.format} />
      <span className="viewer-folder">{entry.relativeDir || t('timeline.rootFolder')}</span>
    </p>
  );
}

/** The photo's info, with the accept, reject, undo and close buttons. */
function ViewerBar({
  entry,
  photo,
  acceptedIds,
  onToggleAccept,
  onReject,
  rejectTo,
  canUndo,
  onUndo,
  onClose,
}: { entry: TimelineEntry } & Pick<
  PhotoViewerProps,
  | 'photo'
  | 'acceptedIds'
  | 'onToggleAccept'
  | 'onReject'
  | 'rejectTo'
  | 'canUndo'
  | 'onUndo'
  | 'onClose'
>) {
  const { t } = useTranslation();
  return (
    <header className="viewer-bar">
      <PhotoInfo entry={entry} details={photo.details} />
      <AcceptButton accepted={acceptedIds.has(entry.id)} onToggle={onToggleAccept} />
      <RejectButton rejectTo={rejectTo} onReject={onReject} />
      <button
        type="button"
        className="viewer-close"
        title={t('undo.hint')}
        disabled={!canUndo}
        onClick={onFirstClick(onUndo)}
      >
        {t('undo.button')}
      </button>
      <button type="button" className="viewer-close" onClick={onFirstClick(onClose)}>
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
  onShow: (id: number) => void;
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

/** Full-size photo with previous/next navigation and the thumbnails of its series. */
export function PhotoViewer(props: PhotoViewerProps) {
  const { photo, scanId, onShow } = props;
  const { entry, previous, next } = photo;
  const { t } = useTranslation();

  useEffect(() => {
    void setWindowTitle(`${entry.name} – Sito`);
  }, [entry.name]);
  useViewerKeys(previous, next, props);
  usePreload(scanId, next, previous);
  const { viewRef, view: zoom } = usePhotoZoom(photo);

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
          <FullPhoto
            src={photoUrl(scanId, entry.id)}
            alt={entry.name}
            zoom={zoom}
            viewRef={viewRef}
          />
        </div>
        <StepButton target={next} label={t('viewer.next')} symbol="›" side="next" onShow={onShow} />
        {photo.series.length > 0 && (
          <SeriesStrip
            members={photo.series}
            currentId={entry.id}
            scanId={scanId}
            acceptedIds={props.acceptedIds}
            onShow={onShow}
          />
        )}
      </div>
    </main>
  );
}
