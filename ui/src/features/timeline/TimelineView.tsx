import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { ScanSummary, Series } from '../../api/scan';
import { loadGap, MAX_GAP, MIN_GAP, saveGap } from './gapSetting';
import type { TimelineMode } from './layout';
import { TimelineGrid } from './TimelineGrid';
import { useTimelineEntries } from './useTimelineEntries';
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
      <span className="timeline-gap-value" aria-hidden="true">
        {t('gap.value', { seconds: gap })}
      </span>
    </div>
  );
}

/** Full-window view of a finished scan: summary bar and the timeline grid. */
export function TimelineView({ path, summary, busy, onChooseAnother }: TimelineViewProps) {
  const { t } = useTranslation();
  // Each picked folder keeps its own gap; another folder starts from its stored one.
  const [chosen, setChosen] = useState<{ path: string; gap: number } | null>(null);
  const gap = chosen?.path === path ? chosen.gap : loadGap(path);
  // A new summary object means a new scan finished, so the timeline is reloaded.
  const timeline = useTimelineEntries(summary, gap);
  const [mode, setMode] = useState<TimelineMode>('series');

  const changeGap = (seconds: number) => {
    setChosen({ path, gap: seconds });
    saveGap(path, seconds);
  };

  return (
    <main className="timeline-view">
      <header className="timeline-bar">
        <h1>{t('app.title')}</h1>
        <div className="timeline-bar-info">
          <p className="timeline-bar-path" title={path}>
            {path}
          </p>
          <p role="status">
            {t('scan.result', { photos: t('scan.photoCount', { count: summary.photosFound }) })}
            {timeline.status === 'ready' && <SeriesSummary series={timeline.series} />}
            {summary.withoutExifDate > 0 &&
              ` ${t('scan.withoutExifDate', { count: summary.withoutExifDate })}`}
            {summary.unreadableEntries > 0 &&
              ` ${t('scan.unreadable', { count: summary.unreadableEntries })}`}
          </p>
        </div>
        <GapSlider gap={gap} onChange={changeGap} />
        <ModeToggle mode={mode} onChange={setMode} />
        <button type="button" disabled={busy} onClick={onChooseAnother}>
          {t('scan.chooseAnother')}
        </button>
      </header>

      {timeline.status === 'loading' && <p className="timeline-message">{t('timeline.loading')}</p>}
      {timeline.status === 'error' && (
        <p role="alert" className="timeline-message">
          {t('timeline.loadError')}
        </p>
      )}
      {timeline.status === 'ready' && mode === 'series' && timeline.series.length === 0 ? (
        <div className="timeline-message">
          <p>{t('series.none')}</p>
          <p>{t('series.noneHint')}</p>
        </div>
      ) : (
        timeline.status === 'ready' && (
          // The grid stays mounted across mode and gap changes and keeps the visible photos in place.
          <TimelineGrid
            entries={timeline.entries}
            series={timeline.series}
            mode={mode}
            scanId={summary.scanId}
          />
        )
      )}
    </main>
  );
}
