import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { rejectPhoto, type Undone } from '../reject/api';
import type { Series, TimelineEntry } from '../timeline/api';
import {
  closeWindow,
  onViewerContext,
  onViewerShow,
  takeViewerRequest,
  type ViewerRequest,
} from './api';
import { Toast } from '../reject/Toast';
import { useUndo } from '../reject/useUndo';
import { useTimelineEntries } from '../timeline/useTimelineEntries';
import { afterReject } from './navigation';
import { PhotoViewer } from './PhotoViewer';

/** Photo id meaning "no photo left to show". */
const NO_PHOTO = -1;

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
  entries: readonly TimelineEntry[];
  series: readonly Series[];
  onShowId: (id: number) => void;
  undo: UndoControls;
}

/**
 * Rejects the photo at `position` and shows the one {@link afterReject}
 * picks, reporting the outcome in a toast. Ignores calls while one is running.
 */
function useReject({ request, entries, series, onShowId, undo }: ReadyViewerProps) {
  const { t } = useTranslation();
  const rejecting = useRef(false);
  const { showToast } = undo;
  return async (position: number) => {
    const entry = entries[position];
    if (!entry || rejecting.current) return;
    rejecting.current = true;
    const after = afterReject(entries.length, series, request.mode, position);
    try {
      const rejected = await rejectPhoto(entry.id);
      onShowId(after === null ? NO_PHOTO : (entries[after]?.id ?? NO_PHOTO));
      showToast({ text: t('reject.done', { name: rejected.name }), kind: 'undoable' });
    } catch {
      showToast({ text: t('reject.failed'), kind: 'error' });
    } finally {
      rejecting.current = false;
    }
  };
}

/** The viewer once the timeline has loaded. */
function ReadyViewer(props: ReadyViewerProps) {
  const { request, entries, series, onShowId, undo } = props;
  const reject = useReject(props);
  const close = useCallback(() => void closeWindow(), []);
  const position = entries.findIndex((entry) => entry.id === request.id);

  if (position >= 0) {
    return (
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
        onReject={() => void reject(position)}
        canUndo={undo.canUndo}
        onUndo={() => void undo.undo()}
      />
    );
  }
  // A restored photo whose timeline is still reloading.
  if (request.id !== NO_PHOTO) return <main className="viewer" />;
  return <EmptyViewer canUndo={undo.canUndo} undo={undo.undo} />;
}

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
  // A restored photo is shown right away.
  const showUndone = useCallback(
    (undone: Undone) => {
      if (undone.id !== null) onShowId(undone.id);
    },
    [onShowId],
  );
  const undo = useUndo(showUndone);

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
  return (
    <>
      <ReadyViewer
        request={request}
        entries={timeline.entries}
        series={timeline.series}
        onShowId={onShowId}
        undo={undo}
      />
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

  if (!request) return <main className="viewer" />;
  return <LoadedViewer request={request} onShowId={showId} />;
}
