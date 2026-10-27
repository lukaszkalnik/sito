import { useTranslation } from 'react-i18next';

import type { ScanSummary } from '../../api/scan';
import { TimelineGrid } from './TimelineGrid';
import { useTimelineEntries } from './useTimelineEntries';
import './TimelineView.css';

interface TimelineViewProps {
  path: string;
  summary: ScanSummary;
  busy: boolean;
  onChooseAnother: () => void;
}

/** Full-window view of a finished scan: summary bar and the timeline grid. */
export function TimelineView({ path, summary, busy, onChooseAnother }: TimelineViewProps) {
  const { t } = useTranslation();
  // A new summary object means a new scan finished, so the timeline is reloaded.
  const timeline = useTimelineEntries(summary);

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
            {summary.withoutExifDate > 0 &&
              ` ${t('scan.withoutExifDate', { count: summary.withoutExifDate })}`}
            {summary.unreadableEntries > 0 &&
              ` ${t('scan.unreadable', { count: summary.unreadableEntries })}`}
          </p>
        </div>
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
      {timeline.status === 'ready' && (
        <TimelineGrid entries={timeline.entries} scanId={summary.scanId} />
      )}
    </main>
  );
}
