import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

import { ChooseFolderButton } from '../scan/ChooseFolderButton';
import type { ScanSummary } from '../scan/api';
import { CutPath } from '../../paths/CutPath';
import type { Series } from './api';
import type { TimelineEntriesState } from './useTimelineEntries';
import './FolderInfo.css';

/** "128 serii (512 zdjęć)." for the summary line. */
function seriesSummary(series: readonly Series[], t: TFunction): string {
  const photos = series.reduce((sum, s) => sum + s.len, 0);
  return t('series.summary', {
    series: t('series.count', { count: series.length }),
    photos: t('scan.photoCount', { count: photos }),
  });
}

/** The full summary sentence and its short form („34 296 zdjęć · 3600 serii”). */
function summaryTexts(summary: ScanSummary, timeline: TimelineEntriesState, t: TFunction) {
  // Rejected photos leave the timeline, so the count follows it once loaded.
  const ready = timeline.status === 'ready' ? timeline : null;
  const photos = t('scan.photoCount', { count: ready?.entries.length ?? summary.photosFound });
  const full = [
    t('scan.result', { photos }),
    ready === null ? null : seriesSummary(ready.series, t),
    summary.withoutDate > 0 ? t('scan.withoutDate', { count: summary.withoutDate }) : null,
    summary.unreadableEntries > 0
      ? t('scan.unreadable', { count: summary.unreadableEntries })
      : null,
  ]
    .filter((part) => part !== null)
    .join(' ');
  const short =
    ready === null
      ? photos
      : t('scan.short', { photos, series: t('series.count', { count: ready.series.length }) });
  return { full, short };
}

/**
 * The summary line: photo and series counts on screen, the full sentence
 * (also photos without a date and unreadable entries) as tooltip and for
 * screen readers.
 */
function ScanStatus({
  summary,
  timeline,
}: {
  summary: ScanSummary;
  timeline: TimelineEntriesState;
}) {
  const { t } = useTranslation();
  const { full, short } = summaryTexts(summary, timeline, t);
  return (
    <p role="status" title={full}>
      <span className="visually-hidden">{full}</span>
      <span aria-hidden="true">{short}</span>
    </p>
  );
}

/** The folder button, then the folder path and the summary line. */
export function FolderInfo({
  path,
  summary,
  timeline,
  busy,
  onChooseAnother,
}: {
  path: string;
  summary: ScanSummary;
  timeline: TimelineEntriesState;
  busy: boolean;
  onChooseAnother: () => void;
}) {
  return (
    <div className="folder-info">
      <ChooseFolderButton busy={busy} onClick={onChooseAnother} />
      <div className="folder-info-text">
        <p>
          <CutPath path={path} />
        </p>
        <ScanStatus summary={summary} timeline={timeline} />
      </div>
    </div>
  );
}
