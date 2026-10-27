import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { rejectPhoto, type Undone } from '../../api/reject';
import {
  closeWindow,
  onViewerContext,
  onViewerShow,
  takeViewerRequest,
  type ViewerRequest,
} from '../../api/viewer';
import { Toast } from '../reject/Toast';
import { useUndo } from '../reject/useUndo';
import { useTimelineEntries } from '../timeline/useTimelineEntries';
import { afterReject } from './navigation';
import { PhotoViewer } from './PhotoViewer';

/** Photo id meaning "no photo left to show". */
const NO_PHOTO = -1;

/** Loads the scan the request belongs to and shows its photo. */
function LoadedViewer({
  request,
  onShowId,
}: {
  request: ViewerRequest;
  onShowId: (id: number) => void;
}) {
  const { t } = useTranslation();
  const timeline = useTimelineEntries(request.scanId, request.gap);
  const close = useCallback(() => void closeWindow(), []);
  // A restored photo is shown right away.
  const showUndone = useCallback(
    (undone: Undone) => {
      if (undone.id !== null) onShowId(undone.id);
    },
    [onShowId],
  );
  const { canUndo, undo, toast, showToast } = useUndo(showUndone);
  const rejecting = useRef(false);

  if (timeline.status === 'loading') return <main className="viewer" />;
  if (timeline.status === 'error') {
    return (
      <main className="viewer">
        <p role="alert" className="viewer-error">
          {t('viewer.loadError')}
        </p>
      </main>
    );
  }
  const { entries, series } = timeline;
  const position = entries.findIndex((entry) => entry.id === request.id);
  const entry = entries[position];

  const reject = async () => {
    if (!entry || rejecting.current) return;
    rejecting.current = true;
    const after = afterReject(entries.length, series, request.mode, position);
    try {
      const rejected = await rejectPhoto(entry.id);
      onShowId(after === null ? NO_PHOTO : (entries[after]?.id ?? NO_PHOTO));
      showToast({ text: t('reject.done', { name: rejected.name }), offerUndo: true });
    } catch {
      showToast({ text: t('reject.failed'), error: true });
    } finally {
      rejecting.current = false;
    }
  };

  return (
    <>
      {entry ? (
        <PhotoViewer
          entries={entries}
          series={series}
          mode={request.mode}
          scanId={request.scanId}
          position={position}
          onShow={(next) => {
            const shown = entries[next];
            if (shown) onShowId(shown.id);
          }}
          onClose={close}
          onReject={() => void reject()}
          canUndo={canUndo}
          onUndo={() => void undo()}
        />
      ) : request.id !== NO_PHOTO ? (
        // A restored photo whose timeline is still reloading.
        <main className="viewer" />
      ) : (
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
      )}
      <Toast message={toast} onUndo={() => void undo()} />
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

  if (!request) return <main className="viewer" />;
  return <LoadedViewer request={request} onShowId={showId} />;
}
