import { useTranslation } from 'react-i18next';

import type { ScanProgress, ScanSummary } from './api';
import { TimelineView } from '../timeline/TimelineView';
import type { ScanState } from './scanReducer';
import { useScan } from './useScan';
import './ScanScreen.css';

/** The finished scan to show, if any (also while picking another folder). */
function finishedScan(state: ScanState): { path: string; summary: ScanSummary } | null {
  if (state.status === 'done') return state;
  if (state.status === 'picking' && state.previous.status === 'done') return state.previous;
  return null;
}

function ProgressText({ progress }: { progress: ScanProgress }) {
  const { t } = useTranslation();
  return (
    <p role="status">
      {progress.phase === 'searching'
        ? t('scan.searching', { photos: t('scan.photoCount', { count: progress.found }) })
        : t('scan.reading', { done: progress.done, total: progress.total })}
    </p>
  );
}

export function ScanScreen() {
  const { t } = useTranslation();
  const { state, chooseAndScan } = useScan();

  const busy = state.status === 'picking' || state.status === 'scanning';
  const finished = finishedScan(state);

  if (finished && finished.summary.photosFound > 0) {
    return (
      <TimelineView
        path={finished.path}
        summary={finished.summary}
        busy={busy}
        onChooseAnother={() => void chooseAndScan()}
      />
    );
  }

  return (
    <main className="start">
      <h1>{t('app.title')}</h1>
      <p>{t('app.tagline')}</p>

      <button type="button" disabled={busy} onClick={() => void chooseAndScan()}>
        {finished || state.status === 'error' ? t('scan.chooseAnother') : t('scan.chooseFolder')}
      </button>

      {'path' in state && <p className="scan-path">{t('scan.folder', { path: state.path })}</p>}

      {state.status === 'scanning' && <ProgressText progress={state.progress} />}

      {finished && (
        <p role="status" className="scan-result">
          {t('scan.noPhotos')}
        </p>
      )}

      {state.status === 'error' && <p role="alert">{t(`scan.errors.${state.code}`)}</p>}
    </main>
  );
}
