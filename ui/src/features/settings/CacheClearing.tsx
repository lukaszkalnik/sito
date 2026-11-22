import { useTranslation } from 'react-i18next';

import { Confirmation } from './Confirmation';
import type { ClearState, ClearTarget } from './useCacheClearing';

const QUESTION: Readonly<Record<ClearTarget, string>> = {
  thumbnails: 'cache.clearThumbnailsConfirm',
  photoData: 'cache.clearPhotoDataConfirm',
};

/** „Wyczyść miniatury” and „Wyczyść dane zdjęć”, each asked first. */
export function CacheClearing({
  state,
  photoDataLocked,
  onAsk,
  onConfirm,
  onCancel,
}: {
  state: ClearState;
  /** A folder is being read: the photo cache must stay. */
  photoDataLocked: boolean;
  onAsk: (target: ClearTarget) => void;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation();
  if (state.kind === 'asking') {
    return (
      <Confirmation
        question={t(QUESTION[state.target])}
        confirmLabel={t('cache.remove')}
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );
  }
  const clearing = state.kind === 'clearing';
  return (
    <div className="cache-section">
      <div className="cache-buttons">
        <button type="button" disabled={clearing} onClick={() => onAsk('thumbnails')}>
          {t('cache.clearThumbnails')}
        </button>
        <button
          type="button"
          disabled={clearing || photoDataLocked}
          title={photoDataLocked ? t('cache.busyHint') : undefined}
          onClick={() => onAsk('photoData')}
        >
          {t('cache.clearPhotoData')}
        </button>
      </div>
      {photoDataLocked && <p className="cache-hint">{t('cache.busyHint')}</p>}
      {state.kind === 'failed' && <p role="alert">{t(`cache.errors.${state.code}`)}</p>}
    </div>
  );
}
