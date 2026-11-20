import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useAcceptedIds } from '../accept/useAcceptedIds';
import { useToggleAccept } from '../accept/useToggleAccept';
import { rejectErrorCode, rejectPhoto, type Undone } from '../reject/api';
import type { RejectDestination } from '../reject/destination';
import { useRejectDestination } from '../reject/useRejectDestination';
import { refreshFolder } from '../scan/api';
import { useShortcuts } from '../../shortcuts/useShortcuts';
import {
  closeWindow,
  onViewerContext,
  onViewerShow,
  currentViewerRequest,
  sendViewerShown,
  type ViewerPhoto,
  type ViewerRequest,
} from './api';
import { Toast } from '../reject/Toast';
import { UndoButton } from '../reject/UndoButton';
import { useUndo } from '../reject/useUndo';
import { HIGHLIGHT_MS } from '../timeline/reveal';
import { LoadingSpinner } from './FullPhoto';
import { PhotoViewer } from './PhotoViewer';
import { EDIT_SHORTCUTS, useViewerSeriesMenu } from './useViewerSeriesMenu';
import { SeriesMenu } from '../series-edit/SeriesMenu';
import { useViewerPhoto } from './useViewerPhoto';

/** Photo id meaning "no photo left to show". */
const NO_PHOTO = -1;

/** Translation keys of the toast after rejecting, per destination. */
const DONE_KEYS: Readonly<Record<RejectDestination, string>> = {
  folder: 'reject.done',
  trash: 'reject.doneTrash',
};

type UndoControls = ReturnType<typeof useUndo>;

/** Shown when every photo of the view was rejected: a message and "Cofnij". */
function EmptyViewer({ undoState, undo }: Pick<UndoControls, 'undoState' | 'undo'>) {
  const { t } = useTranslation();
  return (
    <main className="viewer viewer--empty">
      <p className="viewer-error">{t('viewer.empty')}</p>
      <UndoButton className="control-icon-button" state={undoState} onUndo={() => void undo()} />
    </main>
  );
}

interface ReadyViewerProps {
  request: ViewerRequest;
  photo: ViewerPhoto;
  onShowId: (id: number) => void;
  undo: UndoControls;
  /** The accepted photos of the scan. */
  acceptedIds: ReadonlySet<number>;
  /** Photos of the strip to point out, e.g. those an undone edit changed. */
  pulsedIds: ReadonlySet<number>;
  /** Points out photos of the strip. */
  onPulse: (ids: readonly number[]) => void;
}

/** The translation key of the message for a failed rejection. */
function rejectErrorKey(error: unknown): string {
  const code = rejectErrorCode(error);
  if (code === 'noTrash') return 'reject.noTrash';
  if (code === 'rejectedDirIsLink') return 'reject.rejectedDirIsLink';
  if (code === 'photoMissing') return 'reject.photoMissing';
  return 'reject.failed';
}

/**
 * Rejects the photo shown and shows the one the backend picked for after it,
 * reporting the outcome in a toast. Ignores calls while one is running.
 */
function useReject({ photo, onShowId, undo }: ReadyViewerProps, destination: RejectDestination) {
  const { t } = useTranslation();
  const rejecting = useRef(false);
  const { showToast } = undo;
  return async () => {
    if (rejecting.current) return;
    rejecting.current = true;
    try {
      const rejected = await rejectPhoto(photo.entry.id, destination);
      onShowId(photo.afterReject ?? NO_PHOTO);
      showToast({ text: t(DONE_KEYS[destination], { name: rejected.name }), kind: 'undoable' });
    } catch (error) {
      showToast({ text: t(rejectErrorKey(error)), kind: 'error' });
    } finally {
      rejecting.current = false;
    }
  };
}

/** Closes the window; ignores clicks while it is already closing (a double-click). */
function useCloseOnce() {
  const closing = useRef(false);
  return useCallback(() => {
    if (closing.current) return;
    closing.current = true;
    closeWindow()
      .catch((err: unknown) => console.error('Cannot close the viewer', err))
      .finally(() => {
        closing.current = false;
      });
  }, []);
}

/** Tells the main window which photo is shown, so its grid follows. */
function useAnnounceShown(scanId: number, id: number) {
  useEffect(() => {
    sendViewerShown({ scanId, id }).catch((err: unknown) =>
      console.error('Cannot tell the main window which photo is shown', err),
    );
  }, [scanId, id]);
}

/** The viewer once its photo has loaded. */
function ReadyViewer(props: ReadyViewerProps) {
  const { request, photo, onShowId, undo, acceptedIds, pulsedIds, onPulse } = props;
  useAnnounceShown(request.scanId, photo.entry.id);
  const [destination] = useRejectDestination();
  const reject = useReject(props, destination);
  const toggleAccept = useToggleAccept(acceptedIds, undo.showToast);
  const close = useCloseOnce();
  const seriesMenu = useViewerSeriesMenu({
    photo,
    gap: request.gap,
    showToast: undo.showToast,
    onShow: onShowId,
    onPulse,
  });
  return (
    <PhotoViewer
      photo={photo}
      scanId={request.scanId}
      onShow={onShowId}
      onClose={close}
      onReject={() => void reject()}
      rejectTo={destination}
      acceptedIds={acceptedIds}
      pulsedIds={pulsedIds}
      onToggleAccept={() => void toggleAccept(photo.entry.id)}
      undoState={undo.undoState}
      onUndo={() => void undo.undo()}
      onRefresh={refreshInBackground}
      onMenu={seriesMenu.open}
      onEditSeries={seriesMenu.openBelow}
      editButtonRef={seriesMenu.buttonRef}
      overlay={
        <SeriesMenu
          menu={seriesMenu.menu}
          onChoose={seriesMenu.choose}
          onClose={seriesMenu.close}
          shortcuts={EDIT_SHORTCUTS}
        />
      }
    />
  );
}

/** Rescans the open folder; the main window shows that it runs. */
function refreshInBackground() {
  refreshFolder().catch((err: unknown) => console.error('Cannot refresh the folder', err));
}

/** The viewer while it waits for its photo, or for a restored photo to be back. */
function ViewerLoading() {
  return (
    <main className="viewer">
      <LoadingSpinner />
    </main>
  );
}

/** The viewer with only a message instead of a photo. */
function ViewerMessage({ text }: { text: string }) {
  return (
    <main className="viewer">
      <p role="alert" className="viewer-error">
        {text}
      </p>
    </main>
  );
}

/** Moves on to `next` (as after a rejection) when the photo shown left the timeline. */
function MoveOn({ next, onShowId }: { next: number | null; onShowId: (id: number) => void }) {
  useEffect(() => onShowId(next ?? NO_PHOTO), [next, onShowId]);
  return <ViewerLoading />;
}

/** Loads the photo of `request` and shows it. */
function ViewerPage({
  request,
  onShowId,
  undo,
  acceptedIds,
  pulsedIds,
  onPulse,
}: Omit<ReadyViewerProps, 'photo'>) {
  const { t } = useTranslation();
  const state = useViewerPhoto(request);
  switch (state.kind) {
    case 'loading':
    // A restored photo whose timeline is still reloading.
    case 'missing':
      return <ViewerLoading />;
    case 'gone':
      return <MoveOn next={state.afterReject} onShowId={onShowId} />;
    case 'scanReplaced':
      return <ViewerMessage text={t('viewer.scanReplaced')} />;
    case 'error':
      return <ViewerMessage text={t('viewer.loadError')} />;
    case 'ready':
      return (
        <ReadyViewer
          request={request}
          photo={state.photo}
          onShowId={onShowId}
          undo={undo}
          acceptedIds={acceptedIds}
          pulsedIds={pulsedIds}
          onPulse={onPulse}
        />
      );
  }
}

const NONE_PULSED: ReadonlySet<number> = new Set();

/**
 * The photos to point out in the strip, set with `pulse` and cleared after
 * {@link HIGHLIGHT_MS} (as the grid highlights them).
 */
function usePulsedIds() {
  const [pulsed, setPulsed] = useState<ReadonlySet<number>>(NONE_PULSED);
  useEffect(() => {
    if (pulsed.size === 0) return;
    const timer = setTimeout(() => setPulsed(NONE_PULSED), HIGHLIGHT_MS.restored);
    return () => clearTimeout(timer);
  }, [pulsed]);
  const pulse = useCallback((ids: readonly number[]) => setPulsed(new Set(ids)), []);
  return [pulsed, pulse] as const;
}

/** Shows the photo of `request`, or a message once every photo was rejected. */
function LoadedViewer({
  request,
  onShowId,
}: {
  request: ViewerRequest;
  onShowId: (id: number) => void;
}) {
  const [pulsedIds, pulse] = usePulsedIds();
  // A restored photo is shown right away; after an undone edit the photo
  // shown stays and its strip points out the photos that changed.
  const showUndone = useCallback(
    (undone: Undone) => {
      if (undone.kind === 'edit') pulse(undone.changed);
      else if (undone.id !== null) onShowId(undone.id);
    },
    [onShowId, pulse],
  );
  const undo = useUndo(request.gap, showUndone);
  const acceptedIds = useAcceptedIds(request.scanId);
  // F5 checks the folder here too, instead of reloading the window.
  useShortcuts({ refresh: refreshInBackground });

  return (
    <>
      {request.id === NO_PHOTO ? (
        <EmptyViewer undoState={undo.undoState} undo={undo.undo} />
      ) : (
        <ViewerPage
          request={request}
          onShowId={onShowId}
          undo={undo}
          acceptedIds={acceptedIds}
          pulsedIds={pulsedIds}
          onPulse={pulse}
        />
      )}
      <Toast message={undo.toast} onUndo={() => void undo.undo()} />
    </>
  );
}

/** Root of the viewer window: shows whatever photo the main window asks for. */
export function ViewerApp() {
  const [request, setRequest] = useState<ViewerRequest | null>(null);

  useEffect(() => {
    let active = true;
    const unlisteners = [
      onViewerShow((next) => setRequest(next)),
      onViewerContext((context) => setRequest((current) => current && { ...current, ...context })),
    ];
    // The last request waits in the backend, for a window that was still
    // loading (or reloaded) when it was sent.
    void currentViewerRequest().then((latest) => {
      if (active && latest) setRequest((current) => current ?? latest);
    });
    return () => {
      active = false;
      for (const unlisten of unlisteners) void unlisten.then((stop) => stop());
    };
  }, []);

  const showId = useCallback(
    (id: number) => setRequest((current) => current && { ...current, id }),
    [],
  );

  if (!request) return <ViewerLoading />;
  return <LoadedViewer request={request} onShowId={showId} />;
}
