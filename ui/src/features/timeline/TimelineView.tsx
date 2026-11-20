import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useAcceptedIds } from '../accept/useAcceptedIds';
import type { Undone, UndoState } from '../reject/api';
import type { ScanSummary } from '../scan/api';
import { ScanNotes } from '../scan/ScanNotes';
import { FolderMissingNotice } from '../scan/FolderMissingNotice';
import type { Completing, Missing } from '../scan/scanReducer';
import { useTimelineRevision } from '../reject/useTimelineRevision';
import { useShortcuts } from '../../shortcuts/useShortcuts';
import { openViewer, sendViewerContext } from '../viewer/api';
import { DestinationSetting } from '../reject/DestinationPicker';
import { SeriesMenu } from '../series-edit/SeriesMenu';
import { useSeriesMenu } from '../series-edit/useSeriesMenu';
import { Toast } from '../reject/Toast';
import { UndoButton } from '../reject/UndoButton';
import { useUndo } from '../reject/useUndo';
import { FolderInfo } from './FolderInfo';
import { GapInput } from './GapInput';
import type { TimelineMode } from './layout';
import type { MenuRequest } from './menuRequest';
import { revealOfUndone, type Reveal } from './reveal';
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
  /** What of the folder is known to be gone from disk. */
  missing: Missing;
  /** A folder is being picked or scanned. */
  busy: boolean;
  onChooseAnother: () => void;
  /** Checks the folder for changes in the background. */
  onRefresh: () => void;
}

/**
 * The photo the viewer of scan `scanId` shows, and photos to scroll into view
 * and highlight: what "Cofnij" brought back or changed last, else the photo
 * the viewer showed when it closed.
 */
function useReveal(scanId: number) {
  const [undone, setUndone] = useState<Reveal | null>(null);
  const { followed, lastShown, revealed: lastShownRevealed } = useFollowViewer(scanId);
  const showUndone = useCallback((what: Undone) => setUndone(revealOfUndone(what)), []);
  const revealed = useCallback(() => {
    setUndone(null);
    lastShownRevealed();
  }, [lastShownRevealed]);
  return { reveal: revealOf(undone, lastShown), followed, showUndone, revealed };
}

/** What to reveal: what "Cofnij" brought back first, else the photo the closed viewer showed. */
function revealOf(undone: Reveal | null, lastShown: number | null): Reveal | null {
  if (undone !== null) return undone;
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
  /** Counts the timeline's changes; failed thumbnails load again with each. */
  revision: number;
  /** Opens the edit menu of a photo or series. */
  onMenu: (request: MenuRequest) => void;
}

/** Below the bar: the grid, or a message while loading, on error or without series. */
function TimelineBody(props: TimelineBodyProps) {
  const { timeline, mode, gap, scanId, acceptedIds, reveal, onRevealed, followed } = props;
  const { savedPosition, revision, onMenu } = props;
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
          pinnedIds={timeline.pinnedIds}
          reveal={reveal}
          onRevealed={onRevealed}
          followed={followed}
          savedPosition={savedPosition}
          revision={revision}
          onMenu={onMenu}
          onOpen={(entry) =>
            void openViewer({ id: entry.id, scanId, mode, gap }).catch((err: unknown) =>
              console.error('Cannot open the viewer', err),
            )
          }
        />
      );
  }
}

/**
 * „Odśwież” and F5, unless a folder is being picked or scanned or the open
 * one is refreshing. F5 is taken also then, so it never reloads the page.
 */
function useRefresh(summary: ScanSummary, busy: boolean, onRefresh: () => void) {
  const refreshBusy = busy || summary.refreshing;
  const refresh = useCallback(() => {
    if (!refreshBusy) onRefresh();
  }, [refreshBusy, onRefresh]);
  useShortcuts({ refresh });
  return refresh;
}

interface TimelineBarProps {
  path: string;
  summary: ScanSummary;
  timeline: TimelineEntriesState;
  busy: boolean;
  onChooseAnother: () => void;
  onRefresh: () => void;
  mode: TimelineMode;
  setMode: (mode: TimelineMode) => void;
  gap: number;
  changeGap: (gap: number) => void;
  undoState: UndoState;
  undo: () => Promise<void>;
}

/** The bar above the grid: title, folder, summary and the controls. */
function TimelineBar(props: TimelineBarProps) {
  const { path, summary, timeline, busy, onChooseAnother, onRefresh } = props;
  const { t } = useTranslation();
  return (
    <header className="timeline-bar">
      <h1>{t('app.title')}</h1>
      <FolderInfo {...{ path, summary, timeline, busy, onChooseAnother, onRefresh }} />
      <div className="timeline-bar-controls">
        <GapInput gap={props.gap} onChange={props.changeGap} />
        <ViewPicker mode={props.mode} onChange={props.setMode} />
        <DestinationSetting />
        <UndoButton
          className="control-icon-button"
          state={props.undoState}
          onUndo={() => void props.undo()}
        />
      </div>
    </header>
  );
}

/** Full-window view of a finished scan: summary bar and the timeline grid. */
export function TimelineView(props: TimelineViewProps) {
  const { path, summary, completing, missing, busy, onChooseAnother, onRefresh } = props;
  const [gap, changeGap] = useGap(path);
  // A new scan id means another scan finished, so the timeline is reloaded; a
  // refreshed summary keeps the id and the view.
  const timeline = useTimelineEntries(summary.scanId, gap);
  const [mode, setMode] = useMode(path);
  const savedPosition = useFolderPosition(path);
  const { reveal, followed, showUndone, revealed } = useReveal(summary.scanId);
  const { undoState, undo, toast, showToast } = useUndo(gap, showUndone);
  const seriesMenu = useSeriesMenu(timeline.status === 'ready' ? timeline : null, gap, showToast);
  const acceptedIds = useAcceptedIds(summary.scanId);
  const revision = useTimelineRevision();
  const refresh = useRefresh(summary, busy, onRefresh);
  useViewerContext(mode, gap);

  return (
    <main className="timeline-view">
      <TimelineBar
        {...{ path, summary, timeline, busy, onChooseAnother, mode, setMode, gap, changeGap }}
        onRefresh={refresh}
        undo={undo}
        undoState={undoState}
      />
      {missing === 'folder' && <FolderMissingNotice onChooseAnother={onChooseAnother} />}
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
        revision={revision}
        onMenu={seriesMenu.open}
      />
      <SeriesMenu
        menu={seriesMenu.menu}
        onChoose={(option) => void seriesMenu.choose(option)}
        onClose={seriesMenu.close}
      />
      <Toast message={toast} onUndo={() => void undo()} />
      <ScanNotes
        refreshing={summary.refreshing}
        completing={completing}
        photosMissing={missing === 'photos'}
        onRefresh={refresh}
      />
    </main>
  );
}
