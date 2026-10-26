import { useTranslation } from 'react-i18next';

import { useScan } from './useScan';

export function ScanScreen() {
  const { t } = useTranslation();
  const { state, chooseAndScan } = useScan();

  const busy = state.status === 'picking' || state.status === 'scanning';
  const hasResult = state.status === 'done' || state.status === 'error';

  return (
    <section className="scan">
      <button type="button" disabled={busy} onClick={() => void chooseAndScan()}>
        {hasResult ? t('scan.chooseAnother') : t('scan.chooseFolder')}
      </button>

      {'path' in state && <p className="scan-path">{t('scan.folder', { path: state.path })}</p>}

      {state.status === 'scanning' && (
        <p role="status">
          {t('scan.progress', {
            entries: t('scan.entryCount', { count: state.progress.entriesSeen }),
            photos: t('scan.photoCount', { count: state.progress.photosFound }),
          })}
        </p>
      )}

      {state.status === 'done' && (
        <div role="status">
          <p className="scan-result">
            {state.summary.photosFound === 0
              ? t('scan.noPhotos')
              : t('scan.result', {
                  photos: t('scan.photoCount', { count: state.summary.photosFound }),
                })}
          </p>
          {state.summary.unreadableEntries > 0 && (
            <p>{t('scan.unreadable', { count: state.summary.unreadableEntries })}</p>
          )}
        </div>
      )}

      {state.status === 'error' && <p role="alert">{t(`scan.errors.${state.code}`)}</p>}
    </section>
  );
}
