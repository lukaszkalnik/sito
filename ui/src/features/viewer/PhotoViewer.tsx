import { type ReactNode, useEffect } from 'react';
import { useTranslation } from 'react-i18next';

import { photoUrl, setWindowTitle, type ViewerPhoto } from './api';
import { useShortcuts } from '../../shortcuts/useShortcuts';
import { menuRequest, type MenuRequest } from '../timeline/menuRequest';
import { FullPhoto } from './FullPhoto';
import { SeriesStrip } from './SeriesStrip';
import { usePhotoZoom, type PhotoZoom } from './usePhotoZoom';
import { ViewerBar, type ViewerBarProps } from './ViewerBar';
import './PhotoViewer.css';

interface PhotoViewerProps extends ViewerBarProps {
  /** The photo shown, with its series and neighbours. */
  photo: ViewerPhoto;
  scanId: number;
  /** Shows the photo with the given id. */
  onShow: (id: number) => void;
  /** Checks the open folder for changes, e.g. after the photo's file vanished. */
  onRefresh: () => void;
  /** Photos of the strip to point out, e.g. those an undone edit changed. */
  pulsedIds?: ReadonlySet<number>;
  /** Asks for the edit menu of a photo (the shown one or one of the strip). */
  onMenu: (request: MenuRequest) => void;
  /** Shown over the viewer, inside its dark colours, e.g. the edit menu. */
  overlay?: ReactNode;
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

/** The photo itself; a right-click asks for its edit menu. */
function Stage({
  photo,
  scanId,
  zoom,
  viewRef,
  onRefresh,
  onMenu,
}: Pick<PhotoViewerProps, 'photo' | 'scanId' | 'onRefresh' | 'onMenu'> & {
  zoom: PhotoZoom['view'];
  viewRef: PhotoZoom['viewRef'];
}) {
  const { entry } = photo;
  return (
    <div
      className="viewer-stage"
      onContextMenu={(event) => {
        event.preventDefault();
        onMenu(menuRequest({ kind: 'photo', id: entry.id }, event));
      }}
    >
      <FullPhoto
        src={photoUrl(scanId, entry.id)}
        id={entry.id}
        onRefresh={onRefresh}
        alt={entry.name}
        zoom={zoom}
        viewRef={viewRef}
      />
    </div>
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
  const zoom = usePhotoZoom(photo);

  return (
    <main className="viewer">
      <ViewerBar entry={entry} details={photo.details} pinned={photo.pinned} {...props} />
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
        <Stage
          photo={photo}
          scanId={scanId}
          onRefresh={props.onRefresh}
          onMenu={props.onMenu}
          viewRef={zoom.viewRef}
          zoom={zoom.view}
        />
        <StepButton target={next} label={t('viewer.next')} symbol="›" side="next" onShow={onShow} />
        {photo.series.length > 0 && (
          <SeriesStrip
            members={photo.series}
            currentId={entry.id}
            scanId={scanId}
            acceptedIds={props.acceptedIds}
            pulsedIds={props.pulsedIds}
            edited={photo.seriesEdited}
            onShow={onShow}
            onMenu={props.onMenu}
          />
        )}
      </div>
      {props.overlay}
    </main>
  );
}
