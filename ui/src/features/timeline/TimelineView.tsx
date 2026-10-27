import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { ScanSummary, Series } from '../../api/scan';
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

/** Full-window view of a finished scan: summary bar and the timeline grid. */
export function TimelineView({ path, summary, busy, onChooseAnother }: TimelineViewProps) {
  const { t } = useTranslation();
  // A new summary object means a new scan finished, so the timeline is reloaded.
  const timeline = useTimelineEntries(summary);
  const [mode, setMode] = useState<TimelineMode>('series');

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
          // Remounting on a mode change starts the new view at the top.
          <TimelineGrid
            key={mode}
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
