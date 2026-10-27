import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import {
  closeWindow,
  onViewerContext,
  onViewerShow,
  takeViewerRequest,
  type ViewerRequest,
} from '../../api/viewer';
import { useTimelineEntries } from '../timeline/useTimelineEntries';
import { PhotoViewer } from './PhotoViewer';

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
  const position = timeline.entries.findIndex((entry) => entry.id === request.id);
  return (
    <PhotoViewer
      entries={timeline.entries}
      series={timeline.series}
      mode={request.mode}
      scanId={request.scanId}
      position={position}
      onShow={(next) => {
        const entry = timeline.entries[next];
        if (entry) onShowId(entry.id);
      }}
      onClose={close}
    />
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
