import { useCallback, useEffect, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { Undone } from '../reject/api';
import type { ScanSummary } from '../scan/api';
import type { Series } from './api';
import { openViewer, sendViewerContext } from '../viewer/api';
import { Toast } from '../reject/Toast';
import { useUndo } from '../reject/useUndo';
import { loadGap, MAX_GAP, MIN_GAP, saveGap } from './gapSetting';
import type { TimelineMode } from './layout';
import { TimelineGrid } from './TimelineGrid';
import { useTimelineEntries, type TimelineEntriesState } from './useTimelineEntries';
import './TimelineView.css';

interface TimelineViewProps {
  path: string;
  summary: ScanSummary;
  busy: boolean;
  onChooseAnother: () => void;
}

/** "128 serii (512 zdjęć)." for the summary line. */
function SeriesSummary({ series }: { series: readonly Series[] }) {
  const { t } = useTranslation();
  const photos = series.reduce((sum, s) => sum + s.len, 0);
  return (
    <>
      {' '}
      {t('series.summary', {
        series: t('series.count', { count: series.length }),
        photos: t('scan.photoCount', { count: photos }),
      })}
    </>
  );
}

/** Switch between the series view and the full timeline. */
function ModeToggle({
  mode,
  onChange,
}: {
  mode: TimelineMode;
  onChange: (mode: TimelineMode) => void;
}) {
  const { t } = useTranslation();
  const modes: [TimelineMode, string][] = [
    ['series', t('view.series')],
    ['all', t('view.all')],
  ];
  return (
    <div className="timeline-mode" role="group" aria-label={t('view.label')}>
      {modes.map(([value, label]) => (
        <button
          key={value}
          type="button"
          aria-pressed={mode === value}
          onClick={() => onChange(value)}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

/** Slider for the largest time gap between neighbouring photos of a series. */
function GapSlider({ gap, onChange }: { gap: number; onChange: (gap: number) => void }) {
  const { t } = useTranslation();
  const id = useId();
  return (
    <div className="timeline-gap" title={t('gap.hint')}>
      <label htmlFor={id}>{t('gap.label')}</label>
      <button
        type="button"
        className="timeline-gap-step"
        aria-label={t('gap.decrease')}
        disabled={gap <= MIN_GAP}
        onClick={() => onChange(gap - 1)}
      >
        −
      </button>
      <input
        id={id}
        type="range"
        min={MIN_GAP}
        max={MAX_GAP}
        step={1}
        value={gap}
        aria-valuetext={t('gap.value', { seconds: gap })}
        onChange={(event) => onChange(Number(event.target.value))}
      />
      <button
        type="button"
        className="timeline-gap-step"
        aria-label={t('gap.increase')}
        disabled={gap >= MAX_GAP}
        onClick={() => onChange(gap + 1)}
      >
        +
      </button>
      <span className="timeline-gap-value" aria-hidden="true">
        {t('gap.value', { seconds: gap })}
      </span>
    </div>
  );
}

/** The gap of the folder at `path`: the one chosen in this view, else its stored one. */
function useGap(path: string) {
  // Each picked folder keeps its own gap; another folder starts from its stored one.
  const [chosen, setChosen] = useState<{ path: string; gap: number } | null>(null);
  const gap = chosen?.path === path ? chosen.gap : loadGap(path);
  const changeGap = (seconds: number) => {
    setChosen({ path, gap: seconds });
    saveGap(path, seconds);
  };
  return [gap, changeGap] as const;
}

/** A photo to scroll into view and highlight: the one last restored by "Cofnij". */
function useRevealRestored() {
  const [reveal, setReveal] = useState<number | null>(null);
  const showRestored = useCallback((undone: Undone) => setReveal(undone.id), []);
  const revealed = useCallback(() => setReveal(null), []);
  return { reveal, showRestored, revealed };
}

/** The summary line: photo and series counts and scan notes. */
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
  return (
    <p role="status">
      {t('scan.result', { photos: t('scan.photoCount', { count: photos }) })}
      {timeline.status === 'ready' && <SeriesSummary series={timeline.series} />}
      {summary.withoutExifDate > 0 &&
        ` ${t('scan.withoutExifDate', { count: summary.withoutExifDate })}`}
      {summary.unreadableEntries > 0 &&
        ` ${t('scan.unreadable', { count: summary.unreadableEntries })}`}
      {summary.refreshing && ` ${t('scan.refreshing')}`}
    </p>
  );
}

interface TimelineBodyProps {
  timeline: TimelineEntriesState;
  mode: TimelineMode;
  gap: number;
  scanId: number;
  reveal: number | null;
  onRevealed: () => void;
}

/** Below the bar: the grid, or a message while loading, on error or without series. */
function TimelineBody({ timeline, mode, gap, scanId, reveal, onRevealed }: TimelineBodyProps) {
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
          reveal={reveal}
          onRevealed={onRevealed}
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
export function TimelineView({ path, summary, busy, onChooseAnother }: TimelineViewProps) {
  const { t } = useTranslation();
  const [gap, changeGap] = useGap(path);
  // A new scan id means another scan finished, so the timeline is reloaded; a
  // refreshed summary keeps the id and the view.
  const timeline = useTimelineEntries(summary.scanId, gap);
  const [mode, setMode] = useState<TimelineMode>('series');
  const { reveal, showRestored, revealed } = useRevealRestored();
  const { canUndo, undo, toast } = useUndo(showRestored);

  // An open viewer steps through the same photos as this view.
  useEffect(() => {
    void sendViewerContext({ mode, gap }).catch(() => {
      // No viewer window is open; it gets the view when it opens.
    });
  }, [mode, gap]);

  return (
    <main className="timeline-view">
      <header className="timeline-bar">
        <h1>{t('app.title')}</h1>
        <div className="timeline-bar-info">
          <p className="timeline-bar-path" title={path}>
            {path}
          </p>
          <ScanStatus summary={summary} timeline={timeline} />
        </div>
        <GapSlider gap={gap} onChange={changeGap} />
        <ModeToggle mode={mode} onChange={setMode} />
        <button
          type="button"
          title={t('undo.hint')}
          disabled={!canUndo}
          onClick={() => void undo()}
        >
          {t('undo.button')}
        </button>
        <button type="button" disabled={busy} onClick={onChooseAnother}>
          {t('scan.chooseAnother')}
        </button>
      </header>
      <TimelineBody
        timeline={timeline}
        mode={mode}
        gap={gap}
        scanId={summary.scanId}
        reveal={reveal}
        onRevealed={revealed}
      />
      <Toast message={toast} onUndo={() => void undo()} />
    </main>
  );
}
