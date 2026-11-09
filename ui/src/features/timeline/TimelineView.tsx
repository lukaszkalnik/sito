import { useCallback, useEffect, useState } from 'react';
import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

import { useAcceptedIds } from '../accept/useAcceptedIds';
import type { Undone } from '../reject/api';
import type { ScanSummary } from '../scan/api';
import { ChooseFolderButton } from '../scan/ChooseFolderButton';
import { ScanNotes } from '../scan/ScanNotes';
import type { Completing } from '../scan/scanReducer';
import type { Series } from './api';
import { openViewer, sendViewerContext } from '../viewer/api';
import { DestinationSetting } from '../reject/DestinationPicker';
import { Toast } from '../reject/Toast';
import { UndoButton } from '../reject/UndoButton';
import { useUndo } from '../reject/useUndo';
import { GapSlider } from './GapSlider';
import type { TimelineMode } from './layout';
import type { Reveal } from './reveal';
import { TimelineGrid } from './TimelineGrid';
import { useFolderPosition, useGap, useMode } from './useFolderView';
import { useFollowViewer } from './useFollowViewer';
import type { SavedPosition } from './useSavedPosition';
import { useTimelineEntries, type TimelineEntriesState } from './useTimelineEntries';
import { ViewPicker } from './ViewPicker';
import './TimelineView.css';

interface TimelineViewProps {
  path: string;
  summary: ScanSummary;
  /** Progress of reading EXIF in the background, `null` when none is read. */
  completing: Completing | null;
  busy: boolean;
  onChooseAnother: () => void;
}

/** "128 serii (512 zdjęć)." for the summary line. */
function seriesSummary(series: readonly Series[], t: TFunction): string {
  const photos = series.reduce((sum, s) => sum + s.len, 0);
  return t('series.summary', {
    series: t('series.count', { count: series.length }),
    photos: t('scan.photoCount', { count: photos }),
  });
}

/**
 * The photo the viewer of scan `scanId` shows, and a photo to scroll into view
 * and highlight: the one last restored by "Cofnij", else the one the viewer
 * showed when it closed.
 */
function useReveal(scanId: number) {
  const [restored, setRestored] = useState<number | null>(null);
  const { followed, lastShown, revealed: lastShownRevealed } = useFollowViewer(scanId);
  const showRestored = useCallback((undone: Undone) => setRestored(undone.id), []);
  const revealed = useCallback(() => {
    setRestored(null);
    lastShownRevealed();
  }, [lastShownRevealed]);
  return { reveal: revealOf(restored, lastShown), followed, showRestored, revealed };
}

/** The photo to reveal: a restored one first, else the one the closed viewer showed. */
function revealOf(restored: number | null, lastShown: number | null): Reveal | null {
  if (restored !== null) return { id: restored, reason: 'restored' };
  if (lastShown !== null) return { id: lastShown, reason: 'viewerClosed' };
  return null;
}

/** Keeps an open viewer stepping through the same photos as this view. */
function useViewerContext(mode: TimelineMode, gap: number) {
  useEffect(() => {
    void sendViewerContext({ mode, gap }).catch(() => {
      // No viewer window is open; it gets the view when it opens.
    });
  }, [mode, gap]);
}

/** The summary line with photo and series counts; its full text is also its tooltip. */
function ScanStatus({
  summary,
  timeline,
}: {
  summary: ScanSummary;
  timeline: TimelineEntriesState;
}) {
  const { t } = useTranslation();
  // Rejected photos leave the timeline, so the count follows it once loaded.
  const photos = timeline.status === 'ready' ? timeline.entries.length : summary.photosFound;
  const text = [
    t('scan.result', { photos: t('scan.photoCount', { count: photos }) }),
    timeline.status === 'ready' ? seriesSummary(timeline.series, t) : null,
    summary.withoutDate > 0 ? t('scan.withoutDate', { count: summary.withoutDate }) : null,
    summary.unreadableEntries > 0
      ? t('scan.unreadable', { count: summary.unreadableEntries })
      : null,
  ]
    .filter((part) => part !== null)
    .join(' ');
  return (
    <p role="status" title={text}>
      {text}
    </p>
  );
}

/** The folder button, then the folder path and the summary line. */
function FolderInfo({
  path,
  summary,
  timeline,
  busy,
  onChooseAnother,
}: Pick<TimelineViewProps, 'path' | 'summary' | 'busy' | 'onChooseAnother'> & {
  timeline: TimelineEntriesState;
}) {
  return (
    <div className="timeline-bar-info">
      <ChooseFolderButton busy={busy} onClick={onChooseAnother} />
      <div className="timeline-bar-text">
        <p title={path}>{path}</p>
        <ScanStatus summary={summary} timeline={timeline} />
      </div>
    </div>
  );
}

interface TimelineBodyProps {
  timeline: TimelineEntriesState;
  mode: TimelineMode;
  gap: number;
  scanId: number;
  acceptedIds: ReadonlySet<number>;
  reveal: Reveal | null;
  onRevealed: () => void;
  /** The photo the viewer shows, if any. */
  followed: number | null;
  savedPosition: SavedPosition;
}

/** Below the bar: the grid, or a message while loading, on error or without series. */
function TimelineBody(props: TimelineBodyProps) {
  const { timeline, mode, gap, scanId, acceptedIds, reveal, onRevealed, followed, savedPosition } =
    props;
  const { t } = useTranslation();
  switch (timeline.status) {
    case 'loading':
      return <p className="timeline-message">{t('timeline.loading')}</p>;
    case 'error':
      return (
        <p role="alert" className="timeline-message">
          {t('timeline.loadError')}
        </p>
      );
    case 'ready':
      if (mode === 'series' && timeline.series.length === 0) {
        return (
          <div className="timeline-message">
            <p>{t('series.none')}</p>
            <p>{t('series.noneHint')}</p>
          </div>
        );
      }
      // The grid stays mounted across mode and gap changes and keeps the visible photos in place.
      return (
        <TimelineGrid
          entries={timeline.entries}
          series={timeline.series}
          mode={mode}
          scanId={scanId}
          acceptedIds={acceptedIds}
          reveal={reveal}
          onRevealed={onRevealed}
          followed={followed}
          savedPosition={savedPosition}
          onOpen={(entry) =>
            void openViewer({ id: entry.id, scanId, mode, gap }).catch((err: unknown) =>
              console.error('Cannot open the viewer', err),
            )
          }
        />
      );
  }
}

/** Full-window view of a finished scan: summary bar and the timeline grid. */
export function TimelineView({
  path,
  summary,
  completing,
  busy,
  onChooseAnother,
}: TimelineViewProps) {
  const { t } = useTranslation();
  const [gap, changeGap] = useGap(path);
  // A new scan id means another scan finished, so the timeline is reloaded; a
  // refreshed summary keeps the id and the view.
  const timeline = useTimelineEntries(summary.scanId, gap);
  const [mode, setMode] = useMode(path);
  const savedPosition = useFolderPosition(path);
  const { reveal, followed, showRestored, revealed } = useReveal(summary.scanId);
  const { undoState, undo, toast } = useUndo(showRestored);
  const acceptedIds = useAcceptedIds(summary.scanId);
  useViewerContext(mode, gap);

  return (
    <main className="timeline-view">
      <header className="timeline-bar">
        <h1>{t('app.title')}</h1>
        <FolderInfo {...{ path, summary, timeline, busy, onChooseAnother }} />
        <div className="timeline-bar-controls">
          <GapSlider gap={gap} onChange={changeGap} />
          <ViewPicker mode={mode} onChange={setMode} />
          <DestinationSetting />
          <UndoButton
            className="control-icon-button"
            state={undoState}
            onUndo={() => void undo()}
          />
        </div>
      </header>
      <TimelineBody
        timeline={timeline}
        mode={mode}
        gap={gap}
        scanId={summary.scanId}
        acceptedIds={acceptedIds}
        reveal={reveal}
        onRevealed={revealed}
        followed={followed}
        savedPosition={savedPosition}
      />
      <Toast message={toast} onUndo={() => void undo()} />
      <ScanNotes refreshing={summary.refreshing} completing={completing} />
    </main>
  );
}
