import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { rejectErrorCode, rejectPhoto, type Undone } from '../reject/api';
import type { RejectDestination } from '../reject/destination';
import { useRejectDestination } from '../reject/useRejectDestination';
import {
  closeWindow,
  onViewerContext,
  onViewerShow,
  takeViewerRequest,
  type ViewerPhoto,
  type ViewerRequest,
} from './api';
import { Toast } from '../reject/Toast';
import { useUndo } from '../reject/useUndo';
import { LoadingSpinner } from './FullPhoto';
import { PhotoViewer } from './PhotoViewer';
import { useViewerPhoto } from './useViewerPhoto';

/** Photo id meaning "no photo left to show". */
const NO_PHOTO = -1;

/** Translation keys of the toast after rejecting, per destination. */
const DONE_KEYS: Readonly<Record<RejectDestination, string>> = {
  folder: 'reject.done',
  trash: 'reject.doneTrash',
};

/** Translation keys of the "Odrzuć" button hint, per destination. */
const HINT_KEYS: Readonly<Record<RejectDestination, string>> = {
  folder: 'reject.hint',
  trash: 'reject.hintTrash',
};

type UndoControls = ReturnType<typeof useUndo>;

/** Shown when every photo of the view was rejected: a message and "Cofnij". */
function EmptyViewer({ canUndo, undo }: Pick<UndoControls, 'canUndo' | 'undo'>) {
  const { t } = useTranslation();
  return (
    <main className="viewer viewer--empty">
      <p className="viewer-error">{t('viewer.empty')}</p>
      <button
        type="button"
        className="viewer-close"
        disabled={!canUndo}
        onClick={() => void undo()}
      >
        {t('undo.button')}
      </button>
    </main>
  );
}

interface ReadyViewerProps {
  request: ViewerRequest;
  photo: ViewerPhoto;
  onShowId: (id: number) => void;
  undo: UndoControls;
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
      const key = rejectErrorCode(error) === 'noTrash' ? 'reject.noTrash' : 'reject.failed';
      showToast({ text: t(key), kind: 'error' });
    } finally {
      rejecting.current = false;
    }
  };
}

/** The viewer once its photo has loaded. */
function ReadyViewer(props: ReadyViewerProps) {
  const { request, photo, onShowId, undo } = props;
  const { t } = useTranslation();
  const [destination] = useRejectDestination();
  const reject = useReject(props, destination);
  const close = useCallback(() => void closeWindow(), []);
  return (
    <PhotoViewer
      photo={photo}
      scanId={request.scanId}
      onShow={onShowId}
      onClose={close}
      onReject={() => void reject()}
      rejectHint={t(HINT_KEYS[destination])}
      canUndo={undo.canUndo}
      onUndo={() => void undo.undo()}
    />
  );
}

/** The viewer while it waits for its photo, or for a restored photo to be back. */
function ViewerLoading() {
  return (
    <main className="viewer">
      <LoadingSpinner />
    </main>
  );
}

/** Loads the photo of `request` and shows it. */
function ViewerPage({
  request,
  onShowId,
  undo,
}: {
  request: ViewerRequest;
  onShowId: (id: number) => void;
  undo: UndoControls;
}) {
  const { t } = useTranslation();
  const state = useViewerPhoto(request);
  switch (state.kind) {
    case 'loading':
    // A restored photo whose timeline is still reloading.
    case 'missing':
      return <ViewerLoading />;
    case 'error':
      return (
        <main className="viewer">
          <p role="alert" className="viewer-error">
            {t('viewer.loadError')}
          </p>
        </main>
      );
    case 'ready':
      return <ReadyViewer request={request} photo={state.photo} onShowId={onShowId} undo={undo} />;
  }
}

/** Shows the photo of `request`, or a message once every photo was rejected. */
function LoadedViewer({
  request,
  onShowId,
}: {
  request: ViewerRequest;
  onShowId: (id: number) => void;
}) {
  // A restored photo is shown right away.
  const showUndone = useCallback(
    (undone: Undone) => {
      if (undone.id !== null) onShowId(undone.id);
    },
    [onShowId],
  );
  const undo = useUndo(showUndone);

  return (
    <>
      {request.id === NO_PHOTO ? (
        <EmptyViewer canUndo={undo.canUndo} undo={undo.undo} />
      ) : (
        <ViewerPage request={request} onShowId={onShowId} undo={undo} />
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
    // Requests sent while this window was still loading wait in the backend.
    void takeViewerRequest().then((pending) => {
      if (active && pending) setRequest((current) => current ?? pending);
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
